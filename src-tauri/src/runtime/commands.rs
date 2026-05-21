use super::activity_log::{ActivityEntry, ActivityLogger};
use super::context_engine::ContextEngine;
use super::event_bus::{EventBus, EventType, MeowEvent};
use super::module_loader::{ModuleInfo, ModuleLoader};
use super::reaction_engine::ReactionEngine;
use super::state_machine::StateMachine;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tokio::sync::RwLock;

fn dev_log(app: &tauri::AppHandle, tag: &str, tag_class: &str, msg: &str) {
    let _ = app.emit("dev-log", serde_json::json!({
        "tag": tag,
        "tag_class": tag_class,
        "message": msg,
    }));
}

pub struct RuntimeState {
    pub event_bus: EventBus,
    pub state_machine: StateMachine,
    pub module_loader: Arc<RwLock<ModuleLoader>>,
    pub reaction_engine: ReactionEngine,
    pub activity_logger: ActivityLogger,
    pub context_engine: Arc<RwLock<ContextEngine>>,
}

#[tauri::command]
pub fn get_modules(state: State<'_, RuntimeState>) -> Vec<ModuleInfo> {
    let loader = state.module_loader.blocking_read();
    loader.get_modules()
}

#[tauri::command]
pub async fn refresh_modules(state: State<'_, RuntimeState>) -> Result<Vec<ModuleInfo>, String> {
    let mut loader = state.module_loader.write().await;
    let errors = loader.scan_and_load();
    if !errors.is_empty() {
        eprintln!("[modules] Load errors: {:?}", errors);
    }
    Ok(loader.get_modules())
}

#[tauri::command]
pub async fn toggle_module(
    state: State<'_, RuntimeState>,
    id: String,
    enabled: bool,
) -> Result<bool, String> {
    let mut loader = state.module_loader.write().await;
    Ok(loader.toggle_module(&id, enabled))
}

#[tauri::command]
pub fn get_activity_log(
    state: State<'_, RuntimeState>,
    limit: Option<usize>,
) -> Vec<ActivityEntry> {
    state.activity_logger.read_recent(limit.unwrap_or(50))
}

#[tauri::command]
pub async fn emit_test_event(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    event_type: String,
    source: String,
    payload: std::collections::HashMap<String, String>,
) -> Result<Option<String>, String> {
    let ev_type = match event_type.as_str() {
        "active_app_changed" => EventType::ActiveAppChanged,
        "browser_url_changed" => EventType::BrowserUrlChanged,
        "slack_context_changed" => EventType::SlackContextChanged,
        "user_idle" => EventType::UserIdle,
        "user_typing" => EventType::UserTyping,
        _ => return Err(format!("Unknown event type: {}", event_type)),
    };

    let mut event = MeowEvent::new(ev_type, &source);
    for (k, v) in payload {
        event = event.with_payload(&k, &v);
    }

    state.state_machine.handle_event(&event).await;
    state.event_bus.publish(event.clone()).await;

    let loader = state.module_loader.read().await;
    let active_reactions = loader.get_active_reactions();
    let pairs: Vec<_> = active_reactions.iter().map(|(m, r)| (*m, *r)).collect();

    let result = state.reaction_engine.evaluate(&event, &pairs).await;

    if let Some(ref fired) = result {
        state.activity_logger.append(&ActivityEntry {
            ts: event.timestamp,
            source: fired.module_id.clone(),
            event: "module_reaction".to_string(),
            detail: fired.message.clone(),
            delta: None,
        });

        let _ = app.emit("module-reaction", serde_json::json!({
            "module_id": fired.module_id,
            "message": fired.message,
            "priority": fired.priority,
        }));
    }

    Ok(result.map(|r| r.message))
}

fn priority_overrides_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("claude-meow-pet")
        .join("priority_overrides.json")
}

#[tauri::command]
pub fn get_priority_overrides() -> std::collections::HashMap<String, u8> {
    let path = priority_overrides_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    }
}

#[tauri::command]
pub fn set_priority_overrides(overrides: std::collections::HashMap<String, u8>) -> Result<(), String> {
    let path = priority_overrides_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let data = serde_json::to_string_pretty(&overrides).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_screen_time(state: State<'_, RuntimeState>) -> std::collections::HashMap<String, u64> {
    // Return from activity log as a rough estimate (events in last 24h)
    let entries = state.activity_logger.read_recent(500);
    let mut times: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for entry in &entries {
        if entry.event == "app_switch" {
            *times.entry(entry.detail.clone()).or_insert(0) += 5; // each poll = 5 seconds
        }
    }
    times
}

#[tauri::command]
pub async fn context_on_rage(state: State<'_, RuntimeState>) -> Result<(), String> {
    state.context_engine.write().await.on_rage();
    Ok(())
}

#[tauri::command]
pub async fn context_on_chat(state: State<'_, RuntimeState>) -> Result<(), String> {
    state.context_engine.write().await.on_chat();
    Ok(())
}

#[tauri::command]
pub async fn get_pet_context(state: State<'_, RuntimeState>) -> Result<String, String> {
    Ok(state.context_engine.read().await.get_context_string())
}

#[tauri::command]
pub fn emit_dev_log(app: tauri::AppHandle, tag: String, tag_class: String, message: String) {
    let config = crate::config::load_config();
    if config.dev_mode {
        dev_log(&app, &tag, &tag_class, &message);
    }
}

#[tauri::command]
pub fn log_status_change(
    state: State<'_, RuntimeState>,
    source: String,
    detail: String,
    delta: i32,
) {
    let entry = ActivityEntry {
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        source,
        event: "pet_status_changed".to_string(),
        detail,
        delta: Some(delta),
    };
    state.activity_logger.append(&entry);
}

pub async fn start_polling(app: tauri::AppHandle) {
    let state = app.state::<RuntimeState>();
    let bus = state.event_bus.clone();
    let sm = state.state_machine.clone();
    let module_loader = state.module_loader.clone();
    let reaction_engine_bus = state.event_bus.clone();

    tokio::spawn(async move {
        let reaction_engine = ReactionEngine::new(reaction_engine_bus);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

        let mut last_window = String::new();
        let mut last_reaction_time: u64 = 0;
        let min_reaction_gap_ms: u64 = 5_000;

        // Screen time tracking
        let mut app_times: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let mut current_app_start: u64 = 0;
        let mut current_app_name: String = String::new();
        let mut last_screen_time_alert: u64 = 0;

        // Health reminders (every 45 minutes)
        let mut last_health_reminder: u64 = 0;
        let health_interval_secs: u64 = 2700; // 45 minutes
        let health_messages = [
            "不要喝Monster！喝水！💧",
            "坐直！不要驼背！🐱",
            "不要跷二郎腿！换个姿势！🦵",
            "站起来走走！活动一下！🚶",
            "眼睛累了吗？看看远处20秒！👀",
            "肩膀放松～别耸肩！💆",
            "深呼吸～吸气...呼气...🌬",
            "喝口水！你的身体需要水分！💧",
            "脖子转一转！左右看看！",
            "伸个懒腰！放松一下肌肉！🐈",
        ];

        loop {
            interval.tick().await;

            // Health reminder check
            let now_health = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap()
                .as_secs();
            if now_health - last_health_reminder >= health_interval_secs {
                last_health_reminder = now_health;
                let idx = (now_health as usize) % health_messages.len();
                let _ = app.emit("module-reaction", serde_json::json!({
                    "module_id": "health",
                    "message": health_messages[idx],
                    "priority": 9,
                }));
            }

            let raw_window_info = crate::modules::activity::get_active_window();
            let window_info = raw_window_info.trim_end_matches(" -").trim_end_matches(" - ").trim().to_string();
            let dev_mode = crate::config::load_config().dev_mode;



            // Always log current state in dev mode (even without change)
            if dev_mode && window_info == last_window {
                // No change — just heartbeat every 5s showing current app
                // (only log every 30s to avoid spam)
                let now_check = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap()
                    .as_millis() as u64;
                static LAST_HEARTBEAT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let prev = LAST_HEARTBEAT.load(std::sync::atomic::Ordering::Relaxed);
                if now_check - prev > 30_000 {
                    LAST_HEARTBEAT.store(now_check, std::sync::atomic::Ordering::Relaxed);
                    let parts: Vec<&str> = window_info.splitn(2, " - ").collect();
                    let a = parts.first().unwrap_or(&"");
                    dev_log(&app, "POLL", "skip", &format!("still on \"{}\" (no change)", a));
                }
            }

            // Skip our own app
            if window_info.contains("claude-meow-pet") || window_info.contains("ClaudeMeow") {
                continue;
            }

            if window_info != last_window {
                last_window = window_info.clone();

                let parts: Vec<&str> = window_info.splitn(2, " - ").collect();
                let app_name = parts.first().unwrap_or(&"").to_string();
                let window_title = parts.get(1).unwrap_or(&"").to_string();

                // Track screen time per app
                let now_ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap()
                    .as_secs();
                if !current_app_name.is_empty() && current_app_start > 0 {
                    let elapsed = now_ts - current_app_start;
                    *app_times.entry(current_app_name.clone()).or_insert(0) += elapsed;
                }
                current_app_name = app_name.clone();
                current_app_start = now_ts;

                // React if user has been on the same app too long (>4 hours cumulative, skip browsers)
                let is_browser_app = ["Google Chrome", "Safari", "Firefox", "Microsoft Edge", "Arc", "Brave"]
                    .iter().any(|b| app_name.contains(b));
                let total_secs = app_times.get(&app_name).copied().unwrap_or(0);
                if !is_browser_app && total_secs > 14400 && now_ts - last_screen_time_alert > 1800 {
                    last_screen_time_alert = now_ts;
                    let hours = total_secs / 3600;
                    let mins = (total_secs % 3600) / 60;
                    let messages = [
                        format!("你今天在{}上已经{}小时{}分钟了！休息一下吧！🐱", app_name, hours, mins),
                        format!("{}用了{}小时了！眼睛不累吗？👀", app_name, hours),
                        format!("{}打开太久了！站起来走走！🚶", app_name),
                        format!("已经在{}上{}h{}m了...该休息了！💤", app_name, hours, mins),
                    ];
                    let idx = (now_ts as usize) % messages.len();
                    let _ = app.emit("module-reaction", serde_json::json!({
                        "module_id": "screen-time",
                        "message": messages[idx],
                        "priority": 8,
                    }));
                    if dev_mode {
                        dev_log(&app, "TIME", "error", &format!("{}h{}m on \"{}\"", hours, mins, app_name));
                    }
                }

                // Update context engine
                {
                    let state = app.state::<RuntimeState>();
                    let mut ctx = state.context_engine.write().await;
                    ctx.on_app_switch(&app_name);
                }

                if dev_mode {
                    dev_log(&app, "SWITCH", "event", &format!("→ \"{}\"", app_name));
                }

                let event = MeowEvent::new(EventType::ActiveAppChanged, "system")
                    .with_payload("app_name", &app_name)
                    .with_payload("window_title", &window_title);

                sm.handle_event(&event).await;
                bus.publish(event.clone()).await;

                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap()
                    .as_millis() as u64;

                let lower = format!("{} {}", app_name, window_title).to_lowercase();
                let is_browser = ["chrome", "safari", "firefox", "edge", "arc", "brave"]
                    .iter().any(|b| lower.contains(b));

                if lower.contains("slack") {
                    let slack_event = MeowEvent::new(EventType::SlackContextChanged, "system")
                        .with_payload("activity", "active")
                        .with_payload("workspace", "unknown");
                    bus.publish(slack_event).await;
                    if dev_mode {
                        dev_log(&app, "SLACK", "url", "Slack is active");
                    }
                }

                let browser_site = if is_browser {
                    if dev_mode {
                        dev_log(&app, "FETCH", "event", &format!("fetching URL from \"{}\"...", app_name));
                    }
                    if let Some(url) = get_browser_url(&app_name) {
                        let site = extract_site_from_url(&url);
                        if dev_mode {
                            dev_log(&app, "URL", "url", &format!("site=\"{}\" url=\"{}\"", site, url));
                        }
                        Some((site, url))
                    } else {
                        let site = extract_site(&window_title);
                        if dev_mode {
                            dev_log(&app, "URL", "error", &format!("FAILED to get URL from \"{}\" — fallback site=\"{}\"", app_name, site));
                        }
                        Some((site, window_title.clone()))
                    }
                } else {
                    if dev_mode {
                        dev_log(&app, "APP", "event", &format!("app=\"{}\" title=\"{}\"", app_name, window_title));
                    }
                    None
                };

                // Cooldown check — skip reaction firing but not detection
                if now_ms - last_reaction_time < min_reaction_gap_ms {
                    if dev_mode {
                        let remaining = (min_reaction_gap_ms - (now_ms - last_reaction_time)) / 1000;
                        dev_log(&app, "SKIP", "skip", &format!("cooldown {}s remaining", remaining));
                    }
                    continue;
                }

                let fired = if let Some((site, url)) = browser_site {
                    let browser_event = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
                        .with_payload("site", &site)
                        .with_payload("title", &url);
                    bus.publish(browser_event.clone()).await;

                    let loader = module_loader.read().await;
                    let active = loader.get_active_reactions();
                    let pairs: Vec<_> = active.iter().map(|(m, r)| (*m, *r)).collect();
                    let result = reaction_engine.evaluate(&browser_event, &pairs).await;
                    if dev_mode && result.is_none() {
                        dev_log(&app, "MISS", "skip", &format!("no reaction for site=\"{}\"", site));
                    }
                    result
                } else {
                    let loader = module_loader.read().await;
                    let active = loader.get_active_reactions();
                    let pairs: Vec<_> = active.iter().map(|(m, r)| (*m, *r)).collect();
                    let result = reaction_engine.evaluate(&event, &pairs).await;
                    if dev_mode && result.is_none() {
                        dev_log(&app, "MISS", "skip", &format!("no reaction for app=\"{}\"", app_name));
                    }
                    result
                };

                if let Some(fired) = fired {
                    last_reaction_time = now_ms;
                    if dev_mode {
                        dev_log(&app, "FIRE", "reaction", &format!("module=\"{}\" id=\"{}\" msg=\"{}\"", fired.module_id, fired.reaction_id, fired.message));
                    }
                    let _ = app.emit("module-reaction", serde_json::json!({
                        "module_id": fired.module_id,
                        "message": fired.message,
                        "priority": fired.priority,
                    }));
                }
            }
        }
    });
}

fn extract_site(title: &str) -> String {
    let lower = title.to_lowercase();
    let known_sites = [
        "github", "stackoverflow", "youtube", "twitter", "reddit",
        "linkedin", "amazon", "google", "slack", "notion", "figma",
        "gitlab", "bitbucket", "jira", "confluence", "quip",
    ];
    for site in &known_sites {
        if lower.contains(site) {
            return site.to_string();
        }
    }
    title.split(" - ").last()
        .or_else(|| title.split(" — ").last())
        .unwrap_or(title)
        .trim()
        .to_lowercase()
}

fn get_browser_url(app_name: &str) -> Option<String> {
    let script = if app_name.to_lowercase().contains("chrome") {
        r#"tell application "Google Chrome" to get URL of active tab of front window"#
    } else if app_name.to_lowercase().contains("arc") {
        r#"tell application "Arc" to get URL of active tab of front window"#
    } else if app_name.to_lowercase().contains("safari") {
        r#"tell application "Safari" to get URL of front document"#
    } else if app_name.to_lowercase().contains("edge") {
        r#"tell application "Microsoft Edge" to get URL of active tab of front window"#
    } else if app_name.to_lowercase().contains("firefox") {
        // Firefox doesn't support AppleScript URL access
        return None;
    } else if app_name.to_lowercase().contains("brave") {
        r#"tell application "Brave Browser" to get URL of active tab of front window"#
    } else {
        return None;
    };

    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if url.is_empty() { None } else { Some(url) }
        }
        _ => None,
    }
}

fn extract_site_from_url(url: &str) -> String {
    let lower = url.to_lowercase();
    // Amazon internal sites (check first — more specific)
    let amazon_sites = [
        "code.amazon.com", "phonetool.amazon.com", "quip-amazon.com",
        "w.amazon.com", "issues.amazon.com", "sim.amazon.com",
        "pipelines.amazon.com", "broadcast.amazon.com", "kingpin.amazon.com",
        "sage.amazon.com", "mcm.amazon.com", "mcm.amazon.dev",
        "oncall.corp.amazon.com", "apollo.amazon.com",
        "docs.hub.amazon.dev", "sharepoint.com",
        "quicksight.aws.amazon.com", "shepherd.a2z.com",
        "datacentral.a2z.com", "tod.amazon.com",
    ];
    for site in &amazon_sites {
        if lower.contains(site) {
            return site.to_string();
        }
    }
    let known_sites = [
        "github.com", "youtube.com", "twitter.com",
        "reddit.com", "linkedin.com", "bilibili.com",
        "google.com", "figma.com",
    ];
    for site in &known_sites {
        if lower.contains(site) {
            return site.split('.').next().unwrap_or(site).to_string();
        }
    }
    // Extract domain from URL
    if let Some(start) = lower.find("://") {
        let domain_part = &lower[start + 3..];
        if let Some(end) = domain_part.find('/') {
            return domain_part[..end].to_string();
        }
        return domain_part.to_string();
    }
    lower
}
