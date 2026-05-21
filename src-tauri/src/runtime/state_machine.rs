use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::event_bus::{EventBus, EventType, MeowEvent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PetState {
    Idle,
    Coding,
    Social,
    Meeting,
    Sleepy,
    Excited,
    Angry,
}

impl std::fmt::Display for PetState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PetState::Idle => write!(f, "idle"),
            PetState::Coding => write!(f, "coding"),
            PetState::Social => write!(f, "social"),
            PetState::Meeting => write!(f, "meeting"),
            PetState::Sleepy => write!(f, "sleepy"),
            PetState::Excited => write!(f, "excited"),
            PetState::Angry => write!(f, "angry"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetStats {
    pub happiness: i32,
    pub energy: i32,
    pub hunger: i32,
    pub love: i32,
}

impl Default for PetStats {
    fn default() -> Self {
        Self {
            happiness: 70,
            energy: 80,
            hunger: 60,
            love: 50,
        }
    }
}

impl PetStats {
    #[cfg(test)]
    fn clamp_all(&mut self) {
        self.happiness = self.happiness.clamp(0, 100);
        self.energy = self.energy.clamp(0, 100);
        self.hunger = self.hunger.clamp(0, 100);
        self.love = self.love.clamp(0, 100);
    }
}

#[derive(Clone)]
pub struct StateMachine {
    state: Arc<RwLock<PetState>>,
    #[allow(dead_code)]
    stats: Arc<RwLock<PetStats>>,
    event_bus: EventBus,
}

impl StateMachine {
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            state: Arc::new(RwLock::new(PetState::Idle)),
            stats: Arc::new(RwLock::new(PetStats::default())),
            event_bus,
        }
    }

    #[cfg(test)]
    pub fn with_stats(event_bus: EventBus, stats: PetStats) -> Self {
        Self {
            state: Arc::new(RwLock::new(PetState::Idle)),
            stats: Arc::new(RwLock::new(stats)),
            event_bus,
        }
    }

    #[cfg(test)]
    pub async fn current_state(&self) -> PetState {
        self.state.read().await.clone()
    }

    #[cfg(test)]
    pub async fn current_stats(&self) -> PetStats {
        self.stats.read().await.clone()
    }

    pub async fn transition(&self, new_state: PetState) {
        let mut state = self.state.write().await;
        if *state != new_state {
            let old = state.clone();
            *state = new_state.clone();
            drop(state);

            let event = MeowEvent::new(EventType::PetStatusChanged, "state_machine")
                .with_payload("transition", &format!("{} -> {}", old, new_state));
            self.event_bus.publish(event).await;
        }
    }

    #[cfg(test)]
    pub async fn modify_stat(&self, stat: &str, delta: i32) {
        let mut stats = self.stats.write().await;
        let old_value = match stat {
            "happiness" => stats.happiness,
            "energy" => stats.energy,
            "hunger" => stats.hunger,
            "love" => stats.love,
            _ => return,
        };

        match stat {
            "happiness" => stats.happiness += delta,
            "energy" => stats.energy += delta,
            "hunger" => stats.hunger += delta,
            "love" => stats.love += delta,
            _ => return,
        }
        stats.clamp_all();

        let new_value = match stat {
            "happiness" => stats.happiness,
            "energy" => stats.energy,
            "hunger" => stats.hunger,
            "love" => stats.love,
            _ => 0,
        };
        drop(stats);

        if old_value != new_value {
            let event = MeowEvent::new(EventType::PetStatusChanged, "state_machine")
                .with_payload("stat", stat)
                .with_payload("delta", &delta.to_string())
                .with_payload("old", &old_value.to_string())
                .with_payload("new", &new_value.to_string());
            self.event_bus.publish(event).await;
        }
    }

    pub async fn handle_event(&self, event: &MeowEvent) {
        match event.event_type {
            EventType::ActiveAppChanged => {
                let app = event.payload.get("app_name").map(|s| s.as_str()).unwrap_or("");
                let title = event.payload.get("window_title").map(|s| s.as_str()).unwrap_or("");
                let combined = format!("{} {}", app, title).to_lowercase();

                if is_coding_app(&combined) {
                    self.transition(PetState::Coding).await;
                } else if is_social_app(&combined) {
                    self.transition(PetState::Social).await;
                } else if is_meeting_app(&combined) {
                    self.transition(PetState::Meeting).await;
                } else {
                    self.transition(PetState::Idle).await;
                }
            }
            EventType::UserIdle => {
                if let Some(secs) = event.payload.get("idle_seconds") {
                    if let Ok(s) = secs.parse::<u64>() {
                        if s > 300 {
                            self.transition(PetState::Sleepy).await;
                        }
                    }
                }
            }
            EventType::SlackContextChanged => {
                let activity = event.payload.get("activity").map(|s| s.as_str()).unwrap_or("");
                if activity == "huddle" || activity == "call" {
                    self.transition(PetState::Meeting).await;
                }
            }
            _ => {}
        }
    }
}

fn is_coding_app(s: &str) -> bool {
    ["code", "intellij", "xcode", "vim", "nvim", "terminal", "iterm",
     "warp", "cursor", "webstorm", "pycharm", "clion"]
        .iter().any(|app| s.contains(app))
}

fn is_social_app(s: &str) -> bool {
    ["twitter", "reddit", "instagram", "tiktok", "youtube", "bilibili",
     "weibo", "douyin", "xiaohongshu"]
        .iter().any(|site| s.contains(site))
}

fn is_meeting_app(s: &str) -> bool {
    ["zoom", "chime", "meet.google", "webex"]
        .iter().any(|app| s.contains(app))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sm() -> StateMachine {
        let bus = EventBus::new(64);
        StateMachine::new(bus)
    }

    #[tokio::test]
    async fn test_initial_state() {
        let sm = make_sm();
        assert_eq!(sm.current_state().await, PetState::Idle);
    }

    #[tokio::test]
    async fn test_transition_emits_event() {
        let bus = EventBus::new(64);
        let sm = StateMachine::new(bus.clone());
        let mut rx = bus.subscribe();

        sm.transition(PetState::Coding).await;

        let event = rx.recv().await.unwrap();
        assert_eq!(event.event_type, EventType::PetStatusChanged);
        assert!(event.payload.get("transition").unwrap().contains("coding"));
    }

    #[tokio::test]
    async fn test_no_event_on_same_state() {
        let bus = EventBus::new(64);
        let sm = StateMachine::new(bus.clone());
        let mut rx = bus.subscribe();

        sm.transition(PetState::Idle).await;

        // Should not receive anything since we started at Idle
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            rx.recv()
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_modify_stat_emits_event() {
        let bus = EventBus::new(64);
        let sm = StateMachine::new(bus.clone());
        let mut rx = bus.subscribe();

        sm.modify_stat("happiness", 10).await;

        let event = rx.recv().await.unwrap();
        assert_eq!(event.payload.get("stat").unwrap(), "happiness");
        assert_eq!(event.payload.get("delta").unwrap(), "10");
        assert_eq!(event.payload.get("old").unwrap(), "70");
        assert_eq!(event.payload.get("new").unwrap(), "80");
    }

    #[tokio::test]
    async fn test_stat_clamping() {
        let sm = make_sm();

        sm.modify_stat("happiness", 50).await;
        let stats = sm.current_stats().await;
        assert_eq!(stats.happiness, 100);

        sm.modify_stat("happiness", -200).await;
        let stats = sm.current_stats().await;
        assert_eq!(stats.happiness, 0);
    }

    #[tokio::test]
    async fn test_handle_coding_app_event() {
        let sm = make_sm();

        let event = MeowEvent::new(EventType::ActiveAppChanged, "system")
            .with_payload("app_name", "Code")
            .with_payload("window_title", "main.rs");

        sm.handle_event(&event).await;
        assert_eq!(sm.current_state().await, PetState::Coding);
    }

    #[tokio::test]
    async fn test_handle_social_app_event() {
        let sm = make_sm();

        let event = MeowEvent::new(EventType::ActiveAppChanged, "system")
            .with_payload("app_name", "Chrome")
            .with_payload("window_title", "YouTube - Funny Cats");

        sm.handle_event(&event).await;
        assert_eq!(sm.current_state().await, PetState::Social);
    }

    #[tokio::test]
    async fn test_handle_meeting_event() {
        let sm = make_sm();

        let event = MeowEvent::new(EventType::ActiveAppChanged, "system")
            .with_payload("app_name", "zoom.us")
            .with_payload("window_title", "Meeting");

        sm.handle_event(&event).await;
        assert_eq!(sm.current_state().await, PetState::Meeting);
    }

    #[tokio::test]
    async fn test_handle_idle_event() {
        let sm = make_sm();
        sm.transition(PetState::Coding).await;

        let event = MeowEvent::new(EventType::UserIdle, "system")
            .with_payload("idle_seconds", "600");

        sm.handle_event(&event).await;
        assert_eq!(sm.current_state().await, PetState::Sleepy);
    }

    #[tokio::test]
    async fn test_idle_under_threshold_no_transition() {
        let sm = make_sm();
        sm.transition(PetState::Coding).await;

        let event = MeowEvent::new(EventType::UserIdle, "system")
            .with_payload("idle_seconds", "60");

        sm.handle_event(&event).await;
        assert_eq!(sm.current_state().await, PetState::Coding);
    }

    #[tokio::test]
    async fn test_slack_huddle_triggers_meeting() {
        let sm = make_sm();

        let event = MeowEvent::new(EventType::SlackContextChanged, "slack")
            .with_payload("activity", "huddle");

        sm.handle_event(&event).await;
        assert_eq!(sm.current_state().await, PetState::Meeting);
    }
}
