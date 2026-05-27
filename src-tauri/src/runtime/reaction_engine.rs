use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::event_bus::{EventBus, EventType, MeowEvent};
use super::module_loader::{LoadedModule, ModuleStatus, Reaction, ReactionCondition};

#[derive(Debug, Clone)]
pub struct FiredReaction {
    pub module_id: String,
    pub reaction_id: String,
    pub message: String,
    pub priority: u8,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum EvalResult {
    Fired(FiredReaction),
    OnCooldown { module_id: String, reaction_id: String },
    NoMatch,
}

#[derive(Serialize, Deserialize)]
struct PersistedCooldown {
    last_fired: u64,
    cooldown_ms: u64,
}

struct CooldownEntry {
    last_fired: u64,
    cooldown_ms: u64,
}

fn cooldowns_path() -> PathBuf {
    crate::paths::config_dir().join("cooldowns.json")
}

fn load_cooldowns_from_disk() -> HashMap<String, CooldownEntry> {
    let path = cooldowns_path();
    let Ok(data) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let Ok(map) = serde_json::from_str::<HashMap<String, PersistedCooldown>>(&data) else {
        return HashMap::new();
    };
    let now = now_ms();
    map.into_iter()
        .filter(|(_, pc)| now - pc.last_fired < pc.cooldown_ms)
        .map(|(key, pc)| (key, CooldownEntry {
            last_fired: pc.last_fired,
            cooldown_ms: pc.cooldown_ms,
        }))
        .collect()
}

pub struct ReactionEngine {
    event_bus: EventBus,
    cooldowns: Arc<RwLock<HashMap<String, CooldownEntry>>>,
    cooldowns_dirty: Arc<RwLock<bool>>,
    recent_messages: Arc<RwLock<Vec<String>>>,
}

const RECENT_HISTORY_SIZE: usize = 10;

impl ReactionEngine {
    pub fn new(event_bus: EventBus) -> Self {
        let cooldowns = load_cooldowns_from_disk();
        Self {
            event_bus,
            cooldowns: Arc::new(RwLock::new(cooldowns)),
            cooldowns_dirty: Arc::new(RwLock::new(false)),
            recent_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn save_cooldowns(&self) {
        let cooldowns = self.cooldowns.read().await;
        let now = now_ms();
        let map: HashMap<String, PersistedCooldown> = cooldowns.iter()
            .filter(|(_, v)| now - v.last_fired < v.cooldown_ms)
            .map(|(k, v)| (k.clone(), PersistedCooldown {
                last_fired: v.last_fired,
                cooldown_ms: v.cooldown_ms,
            }))
            .collect();
        if let Ok(data) = serde_json::to_string(&map) {
            if let Err(e) = fs::write(cooldowns_path(), data) {
                eprintln!("[ClaudeMeow] cooldowns save failed: {}", e);
            }
        }
    }

    pub async fn evaluate(
        &self,
        event: &MeowEvent,
        modules: &[(&LoadedModule, &Reaction)],
    ) -> Option<FiredReaction> {
        match self.evaluate_detailed(event, modules).await {
            EvalResult::Fired(f) => Some(f),
            _ => None,
        }
    }

    pub async fn evaluate_detailed(
        &self,
        event: &MeowEvent,
        modules: &[(&LoadedModule, &Reaction)],
    ) -> EvalResult {
        let now = now_ms();
        let mut candidates: Vec<FiredReaction> = Vec::new();
        let mut blocked_by_cooldown: Option<(String, String)> = None;

        let overrides = load_priority_overrides();

        for (module, reaction) in modules {
            if module.status != ModuleStatus::Active {
                continue;
            }

            if !module_has_permission_for_event(module, &event.event_type) {
                continue;
            }

            if !self.event_matches_trigger(event, reaction) {
                continue;
            }

            let cooldown_key = format!("{}::{}", module.manifest.id, reaction.id);
            if self.is_on_cooldown(&cooldown_key, now).await {
                if blocked_by_cooldown.is_none() {
                    blocked_by_cooldown = Some((module.manifest.id.clone(), reaction.id.clone()));
                }
                continue;
            }

            let effective_priority = if let Some(&level) = overrides.get(&module.manifest.id) {
                level_to_priority(level)
            } else {
                reaction.response.priority
            };

            let recent = self.recent_messages.read().await;
            let message = pick_avoiding_recent(&reaction.response.messages, &recent);
            drop(recent);
            candidates.push(FiredReaction {
                module_id: module.manifest.id.clone(),
                reaction_id: reaction.id.clone(),
                message,
                priority: effective_priority,
            });
        }

        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
        let winner = candidates.into_iter().next();

        if let Some(ref fired) = winner {
            let cooldown_key = format!("{}::{}", fired.module_id, fired.reaction_id);
            let module_reaction = modules.iter()
                .find(|(m, r)| m.manifest.id == fired.module_id && r.id == fired.reaction_id);
            if let Some((_, reaction)) = module_reaction {
                self.set_cooldown(&cooldown_key, now, reaction.response.cooldown_minutes).await;
            }

            let mut recent = self.recent_messages.write().await;
            recent.push(fired.message.clone());
            if recent.len() > RECENT_HISTORY_SIZE {
                recent.remove(0);
            }
            drop(recent);

            let event = MeowEvent::new(EventType::ModuleReaction, &fired.module_id)
                .with_payload("message", &fired.message)
                .with_payload("priority", &fired.priority.to_string())
                .with_payload("reaction_id", &fired.reaction_id);
            self.event_bus.publish(event).await;

            return EvalResult::Fired(fired.clone());
        }

        if let Some((module_id, reaction_id)) = blocked_by_cooldown {
            EvalResult::OnCooldown { module_id, reaction_id }
        } else {
            EvalResult::NoMatch
        }
    }

    fn event_matches_trigger(&self, event: &MeowEvent, reaction: &Reaction) -> bool {
        let trigger_event = event_type_from_string(&reaction.trigger.event);
        if trigger_event != Some(event.event_type.clone()) {
            return false;
        }

        match &reaction.trigger.condition {
            None => true,
            Some(condition) => self.evaluate_condition(event, condition),
        }
    }

    fn evaluate_condition(&self, event: &MeowEvent, condition: &ReactionCondition) -> bool {
        let field_value = match event.payload.get(&condition.field) {
            Some(v) => v.to_lowercase(),
            None => return false,
        };

        if let Some(ref contains) = condition.contains {
            if !field_value.contains(&contains.to_lowercase()) {
                return false;
            }
        }

        if let Some(ref equals) = condition.equals {
            if field_value != equals.to_lowercase() {
                return false;
            }
        }

        if let Some(ref not_contains) = condition.not_contains {
            if field_value.contains(&not_contains.to_lowercase()) {
                return false;
            }
        }

        true
    }

    async fn is_on_cooldown(&self, key: &str, now: u64) -> bool {
        let cooldowns = self.cooldowns.read().await;
        if let Some(entry) = cooldowns.get(key) {
            now - entry.last_fired < entry.cooldown_ms
        } else {
            false
        }
    }

    async fn set_cooldown(&self, key: &str, now: u64, cooldown_minutes: u32) {
        let mut cooldowns = self.cooldowns.write().await;
        cooldowns.insert(key.to_string(), CooldownEntry {
            last_fired: now,
            cooldown_ms: (cooldown_minutes as u64) * 60 * 1000,
        });
        drop(cooldowns);
        *self.cooldowns_dirty.write().await = true;
    }

    pub async fn flush_if_dirty(&self) {
        let dirty = *self.cooldowns_dirty.read().await;
        if dirty {
            self.save_cooldowns().await;
            *self.cooldowns_dirty.write().await = false;
        }
    }
}

fn load_priority_overrides() -> HashMap<String, u8> {
    let path = crate::paths::config_dir().join("priority_overrides.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn level_to_priority(level: u8) -> u8 {
    match level {
        1 => 9,
        2 => 7,
        3 => 5,
        4 => 2,
        _ => 5,
    }
}

fn module_has_permission_for_event(module: &LoadedModule, event_type: &EventType) -> bool {
    let perms = &module.manifest.permissions;
    if perms.is_empty() {
        return true;
    }
    match event_type {
        EventType::BrowserUrlChanged => perms.iter().any(|p| p == "browser_url"),
        EventType::ActiveAppChanged => perms.iter().any(|p| p == "active_app"),
        EventType::SlackContextChanged => perms.iter().any(|p| p == "active_app" || p == "browser_url"),
        EventType::UserTyping => perms.iter().any(|p| p == "active_app"),
        EventType::UserIdle => perms.iter().any(|p| p == "active_app"),
        _ => true,
    }
}

fn event_type_from_string(s: &str) -> Option<EventType> {
    match s {
        "active_app_changed" => Some(EventType::ActiveAppChanged),
        "browser_url_changed" => Some(EventType::BrowserUrlChanged),
        "slack_context_changed" => Some(EventType::SlackContextChanged),
        "user_idle" => Some(EventType::UserIdle),
        "user_typing" => Some(EventType::UserTyping),
        "pet_status_changed" => Some(EventType::PetStatusChanged),
        "module_loaded" => Some(EventType::ModuleLoaded),
        "module_reaction" => Some(EventType::ModuleReaction),
        _ => None,
    }
}

fn pick_avoiding_recent(messages: &[String], recent: &[String]) -> String {
    if messages.is_empty() {
        return String::new();
    }
    let fresh: Vec<&String> = messages.iter().filter(|m| !recent.contains(m)).collect();
    let pool = if fresh.is_empty() { messages.iter().collect() } else { fresh };
    let idx = rand::thread_rng().gen_range(0..pool.len());
    pool[idx].clone()
}

use crate::util::now_ms;

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::module_loader::*;
    use std::path::PathBuf;

    fn make_module(id: &str, reactions: Vec<Reaction>) -> LoadedModule {
        LoadedModule {
            manifest: ModuleManifest {
                id: id.to_string(),
                name: id.to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                author: String::new(),
                icon: String::new(),
                builtin: false,
                permissions: Vec::new(),
                event_subscriptions: Vec::new(),
            },
            reactions,
            status: ModuleStatus::Active,
            error: None,
            path: PathBuf::from("/tmp/test"),
        }
    }

    fn make_reaction(id: &str, event: &str, field: &str, contains: &str, messages: Vec<&str>, priority: u8) -> Reaction {
        Reaction {
            id: id.to_string(),
            trigger: ReactionTrigger {
                event: event.to_string(),
                condition: if field.is_empty() {
                    None
                } else {
                    Some(ReactionCondition {
                        field: field.to_string(),
                        contains: if contains.is_empty() { None } else { Some(contains.to_string()) },
                        equals: None,
                        not_contains: None,
                    })
                },
            },
            response: ReactionResponse {
                messages: messages.into_iter().map(String::from).collect(),
                priority,
                cooldown_minutes: 0,
            },
        }
    }

    #[tokio::test]
    async fn test_basic_reaction_match() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let module = make_module("github-helper", vec![
            make_reaction("gh", "browser_url_changed", "site", "github", vec!["Coding on GitHub!"], 5),
        ]);

        let event = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
            .with_payload("site", "github.com")
            .with_payload("title", "Pull Request");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().message, "Coding on GitHub!");
    }

    #[tokio::test]
    async fn test_no_match_wrong_event() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let module = make_module("test", vec![
            make_reaction("r1", "browser_url_changed", "site", "github", vec!["GitHub!"], 5),
        ]);

        let event = MeowEvent::new(EventType::UserTyping, "system");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_no_match_wrong_condition() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let module = make_module("test", vec![
            make_reaction("r1", "browser_url_changed", "site", "github", vec!["GitHub!"], 5),
        ]);

        let event = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
            .with_payload("site", "youtube.com");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let module = make_module("multi", vec![
            make_reaction("low", "browser_url_changed", "site", "github", vec!["Low priority"], 2),
            make_reaction("high", "browser_url_changed", "site", "github", vec!["High priority"], 9),
            make_reaction("mid", "browser_url_changed", "site", "github", vec!["Mid priority"], 5),
        ]);

        let event = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
            .with_payload("site", "github.com");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().message, "High priority");
    }

    #[tokio::test]
    async fn test_cooldown_prevents_repeat() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let unique_id = format!("cd-test-{}", now_ms());
        let unique_mod = format!("cd-mod-{}", now_ms());

        let reaction = Reaction {
            id: unique_id,
            trigger: ReactionTrigger {
                event: "user_typing".to_string(),
                condition: None,
            },
            response: ReactionResponse {
                messages: vec!["Typing!".to_string()],
                priority: 5,
                cooldown_minutes: 60,
            },
        };
        let module = make_module(&unique_mod, vec![reaction]);

        let event = MeowEvent::new(EventType::UserTyping, "system");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let first = engine.evaluate(&event, &pairs).await;
        assert!(first.is_some());

        let second = engine.evaluate(&event, &pairs).await;
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_disabled_module_ignored() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let mut module = make_module("disabled", vec![
            make_reaction("r1", "user_typing", "", "", vec!["Should not fire"], 5),
        ]);
        module.status = ModuleStatus::Disabled;

        let event = MeowEvent::new(EventType::UserTyping, "system");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_reaction_without_condition() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let module = make_module("no-cond", vec![
            make_reaction("r1", "user_typing", "", "", vec!["Typing detected!"], 5),
        ]);

        let event = MeowEvent::new(EventType::UserTyping, "system");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().message, "Typing detected!");
    }

    #[tokio::test]
    async fn test_condition_case_insensitive() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let module = make_module("case", vec![
            make_reaction("r1", "browser_url_changed", "site", "GitHub", vec!["Found it!"], 5),
        ]);

        let event = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
            .with_payload("site", "GITHUB.COM");

        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_not_contains_condition() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let reaction = Reaction {
            id: "not-youtube".to_string(),
            trigger: ReactionTrigger {
                event: "browser_url_changed".to_string(),
                condition: Some(ReactionCondition {
                    field: "site".to_string(),
                    contains: None,
                    equals: None,
                    not_contains: Some("youtube".to_string()),
                }),
            },
            response: ReactionResponse {
                messages: vec!["Not YouTube!".to_string()],
                priority: 5,
                cooldown_minutes: 0,
            },
        };
        let module = make_module("nc", vec![reaction]);

        // Should match (not youtube)
        let event = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
            .with_payload("site", "github.com");
        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();
        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_some());

        // Should NOT match (is youtube)
        let event2 = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
            .with_payload("site", "youtube.com");
        let result2 = engine.evaluate(&event2, &pairs).await;
        assert!(result2.is_none());
    }

    #[tokio::test]
    async fn test_equals_condition() {
        let bus = EventBus::new(64);
        let engine = ReactionEngine::new(bus);

        let reaction = Reaction {
            id: "exact".to_string(),
            trigger: ReactionTrigger {
                event: "slack_context_changed".to_string(),
                condition: Some(ReactionCondition {
                    field: "channel_type".to_string(),
                    contains: None,
                    equals: Some("dm".to_string()),
                    not_contains: None,
                }),
            },
            response: ReactionResponse {
                messages: vec!["DM mode!".to_string()],
                priority: 5,
                cooldown_minutes: 0,
            },
        };
        let module = make_module("eq", vec![reaction]);

        let event = MeowEvent::new(EventType::SlackContextChanged, "slack")
            .with_payload("channel_type", "dm");
        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();
        let result = engine.evaluate(&event, &pairs).await;
        assert!(result.is_some());

        let event2 = MeowEvent::new(EventType::SlackContextChanged, "slack")
            .with_payload("channel_type", "channel");
        let result2 = engine.evaluate(&event2, &pairs).await;
        assert!(result2.is_none());
    }

    #[tokio::test]
    async fn test_emits_module_reaction_event() {
        let bus = EventBus::new(64);
        let mut rx = bus.subscribe();
        let engine = ReactionEngine::new(bus);

        let module = make_module("emitter", vec![
            make_reaction("r1", "user_typing", "", "", vec!["Hello!"], 7),
        ]);

        let event = MeowEvent::new(EventType::UserTyping, "system");
        let pairs: Vec<(&LoadedModule, &Reaction)> = module.reactions.iter()
            .map(|r| (&module, r)).collect();

        engine.evaluate(&event, &pairs).await;

        let emitted = rx.recv().await.unwrap();
        assert_eq!(emitted.event_type, EventType::ModuleReaction);
        assert_eq!(emitted.source, "emitter");
        assert_eq!(emitted.payload.get("message").unwrap(), "Hello!");
        assert_eq!(emitted.payload.get("priority").unwrap(), "7");
    }
}
