use super::browser;
use super::calendar_reminder::CalendarReminder;
use super::commands::RuntimeState;
use super::event_bus::{EventType, MeowEvent};
use super::health_reminder::HealthReminder;
use super::reaction_engine::ReactionEngine;
use super::screen_time::ScreenTimeTracker;
use super::secret_meow::SecretMeowState;
use tauri::{Emitter, Manager};

const HEARTBEAT_INTERVAL_MS: u64 = 300_000;
const CONFIG_RELOAD_MS: u64 = 60_000;

fn dev_log(app: &tauri::AppHandle, tag: &str, tag_class: &str, msg: &str) {
    let _ = app.emit("dev-log", serde_json::json!({
        "tag": tag, "tag_class": tag_class, "message": msg,
    }));
}

fn is_dev_mode(app: &tauri::AppHandle) -> bool {
    let config_on = crate::config::load_config().dev_mode;
    let window_visible = app.get_webview_window("dev")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);
    config_on || window_visible
}

fn cat_status_for_app(app_lower: &str) -> &'static str {
    if app_lower.contains("code") || app_lower.contains("intellij") || app_lower.contains("cursor") {
        "🐾 看你写代码..."
    } else if app_lower.contains("terminal") || app_lower.contains("iterm") || app_lower.contains("warp") {
        "💻 盯着终端看..."
    } else if app_lower.contains("zoom") {
        "🤫 安静等你开完会..."
    } else if app_lower.contains("slack") {
        "👀 偷看你聊天..."
    } else if app_lower.contains("spotify") {
        "🎵 跟着音乐摇尾巴..."
    } else if app_lower.contains("chrome") || app_lower.contains("safari") || app_lower.contains("arc") {
        "🌐 看你上网冲浪..."
    } else {
        "😺 好奇地看着..."
    }
}

pub async fn start_polling(app: tauri::AppHandle) {
    let state = app.state::<RuntimeState>();
    let bus = state.event_bus.clone();
    let sm = state.state_machine.clone();
    let module_loader = state.module_loader.clone();
    let reaction_engine_bus = state.event_bus.clone();

    tokio::spawn(async move {
        let reaction_engine = ReactionEngine::new(reaction_engine_bus);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

        let mut last_window = String::new();
        let mut last_app_name = String::new();
        let startup_time = crate::util::now_ms();
        let mut last_reaction_time: u64 = 0;

        let mut health = HealthReminder::new();
        let mut screen_time = ScreenTimeTracker::new();
        let mut calendar = CalendarReminder::new();
        let mut secret_meow = SecretMeowState::try_load();
        super::secret_meow::init_global_context(&secret_meow);

        let mut last_heartbeat: u64 = crate::util::now_ms();
        let mut was_typing = false;
        let mut last_typing_log: u64 = 0;
        let mut last_title_log: u64 = 0;
        let mut last_logged_title = String::new();
        let mut last_periodic_eval: u64 = 0;
        let mut last_config_load: u64 = 0;
        let mut reaction_cooldown_ms: u64 = 10_000;

        loop {
            interval.tick().await;

            let now_secs = crate::util::now_secs();
            let now_ms = crate::util::now_ms();
            let dev_mode = is_dev_mode(&app);

            // Reload config periodically
            if now_ms - last_config_load > CONFIG_RELOAD_MS {
                last_config_load = now_ms;
                let cfg = crate::config::load_config();
                reaction_cooldown_ms = (cfg.reaction_cooldown_secs as u64) * 1000;
                health.update_config(cfg.health_enabled, cfg.health_interval_minutes, cfg.quiet_hours_start, cfg.quiet_hours_end);

                // Load calendar config from amazon-internal module
                let amazon_config_path = super::memory::amazon_data_dir().join("config.json");
                if let Ok(data) = std::fs::read_to_string(&amazon_config_path) {
                    if let Ok(acfg) = serde_json::from_str::<serde_json::Value>(&data) {
                        let cal_enabled = acfg["calendar_enabled"].as_bool().unwrap_or(false);
                        let alias = acfg["user_alias"].as_str().unwrap_or("").to_string();
                        calendar.update_config(cal_enabled, alias);
                    }
                }
            }

            // Daily commits refresh (every 24h, auto-fetch fresh commits)
            {
                let commits_path = super::memory::amazon_data_dir().join("commits.json");
                let stale = commits_path.exists() && std::fs::metadata(&commits_path).ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| t.elapsed().unwrap_or_default().as_secs() > 86400)
                    .unwrap_or(false);
                if stale {
                    let amazon_config_path = super::memory::amazon_data_dir().join("config.json");
                    if let Ok(data) = std::fs::read_to_string(&amazon_config_path) {
                        if let Ok(acfg) = serde_json::from_str::<serde_json::Value>(&data) {
                            let alias = acfg["user_alias"].as_str().unwrap_or("").to_string();
                            let days = acfg["cr_days_range"].as_u64().unwrap_or(14).to_string();
                            if !alias.is_empty() {
                                if let Ok(team) = super::mcp_runner::call_mcp("amazon-internal", &["get-team", "--alias", &alias]) {
                                    if let Some(teammates) = team["teammates"].as_array() {
                                        let mut all: Vec<String> = teammates.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect();
                                        all.push(alias);
                                        let aliases_str = all.join(",");
                                        let _ = super::mcp_runner::call_mcp("amazon-internal", &["get-commits", "--aliases", &aliases_str, "--days", &days]);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Health reminder (delegated)
            health.tick(&app, now_secs);

            // Calendar reminder (delegated)
            calendar.tick(&app, now_secs);

            // Secret meow messages (one at a time from encrypted profile)
            secret_meow.tick(&app, now_secs);

            // Flush cooldowns to disk periodically (batched)
            reaction_engine.flush_if_dirty().await;

            // Poll active window
            let raw_window_info = crate::modules::activity::get_active_window();
            let raw_trimmed = raw_window_info
                .trim_end_matches(" -")
                .trim_end_matches(" - ")
                .trim()
                .to_string();

            let parts_raw: Vec<&str> = raw_trimmed.splitn(2, " - ").collect();
            let app_part = parts_raw.first().unwrap_or(&"");
            let title_part = parts_raw.get(1).unwrap_or(&"");
            let is_self = app_part.eq_ignore_ascii_case("claude-meow-pet")
                || app_part.eq_ignore_ascii_case("ClaudeMeow");
            let window_info = if is_self {
                if last_window.is_empty() { continue; }
                last_window.clone()
            } else if title_part.is_empty() && !last_window.is_empty() && *app_part == last_app_name {
                // Transient blank title within SAME app — keep previous
                last_window.clone()
            } else {
                raw_trimmed
            };

            // Typing detection (throttled: log at most every 60s)
            let typing_now = crate::modules::activity::is_typing();
            if typing_now && !was_typing {
                was_typing = true;
                let typing_event = MeowEvent::new(EventType::UserTyping, "system");
                bus.publish(typing_event).await;
                if dev_mode && now_ms - last_typing_log > 60_000 {
                    last_typing_log = now_ms;
                    dev_log(&app, "TYPE", "event", "keyboard activity detected");
                }
            } else if !typing_now && was_typing {
                was_typing = false;
            }

            // Heartbeat + periodic re-evaluation when no window change
            if window_info == last_window {
                if now_ms - last_heartbeat > HEARTBEAT_INTERVAL_MS {
                    last_heartbeat = now_ms;
                    let parts: Vec<&str> = window_info.splitn(2, " - ").collect();
                    let a = parts.first().unwrap_or(&"");
                    let t = parts.get(1).unwrap_or(&"");
                    if dev_mode {
                        dev_log(&app, "POLL", "skip", &format!(
                            "app=\"{}\" title=\"{}\"", a, &t[..t.len().min(40)]
                        ));
                    }
                    let status = if typing_now {
                        "⌨️ 看你打字ing..."
                    } else {
                        cat_status_for_app(&a.to_lowercase())
                    };
                    let _ = app.emit("cat-status", serde_json::json!({ "status": status }));
                }

                // Re-evaluate reactions every 30s even without app switch
                if now_ms - last_periodic_eval > 30_000
                    && now_ms - last_reaction_time >= reaction_cooldown_ms
                {
                    last_periodic_eval = now_ms;
                    let parts: Vec<&str> = window_info.splitn(2, " - ").collect();
                    let app_name = parts.first().unwrap_or(&"").to_string();
                    let window_title = parts.get(1).unwrap_or(&"").to_string();

                    let event = MeowEvent::new(EventType::ActiveAppChanged, "system")
                        .with_payload("app_name", &app_name)
                        .with_payload("window_title", &window_title);

                    let loader = module_loader.read().await;
                    let active = loader.get_active_reactions();
                    let pairs: Vec<_> = active.iter().map(|(m, r)| (*m, *r)).collect();
                    let result = reaction_engine.evaluate(&event, &pairs).await;

                    if dev_mode {
                        if let Some(ref fired) = result {
                            dev_log(&app, "FIRE", "reaction", &format!(
                                "module=\"{}\" id=\"{}\"", fired.module_id, fired.reaction_id
                            ));
                        }
                    }

                    if let Some(fired) = result {
                        last_reaction_time = now_ms;
                        let _ = app.emit("module-reaction", serde_json::json!({
                            "module_id": fired.module_id,
                            "message": fired.message,
                            "priority": fired.priority,
                        }));
                    }
                }
                continue;
            }

            // ── Window changed ──
            last_window = window_info.clone();

            let parts: Vec<&str> = window_info.splitn(2, " - ").collect();
            let app_name = parts.first().unwrap_or(&"").to_string();
            let window_title = parts.get(1).unwrap_or(&"").to_string();

            let is_app_switch = app_name != last_app_name;

            if is_app_switch {
                last_app_name = app_name.clone();
                last_heartbeat = now_ms;

                // Screen time tracking (delegated)
                screen_time.on_app_switch(&app, &app_name, now_secs, dev_mode);

                // Update context engine
                {
                    let state = app.state::<RuntimeState>();
                    let mut ctx = state.context_engine.write().await;
                    ctx.on_app_switch(&app_name);
                }

                if dev_mode {
                    dev_log(&app, "SWITCH", "event", &format!("→ \"{}\" title=\"{}\"", app_name, &window_title[..window_title.len().min(40)]));
                }
            } else if dev_mode && now_ms - last_title_log > 10_000 && window_info != last_logged_title {
                last_title_log = now_ms;
                last_logged_title = window_info.clone();
                dev_log(&app, "APP", "event", &format!("app=\"{}\" title=\"{}\"", app_name, &window_title[..window_title.len().min(40)]));
            }

            if !is_app_switch {
                continue;
            }

            // ── Reaction evaluation ──
            let event = MeowEvent::new(EventType::ActiveAppChanged, "system")
                .with_payload("app_name", &app_name)
                .with_payload("window_title", &window_title);

            sm.handle_event(&event).await;
            bus.publish(event.clone()).await;

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
                if let Some(url) = browser::get_browser_url(&app_name) {
                    let site = browser::extract_site_from_url(&url);
                    if dev_mode {
                        let short_url = if url.len() > 60 { format!("{}...", &url[..60]) } else { url.clone() };
                        dev_log(&app, "URL", "url", &format!("site=\"{}\" url=\"{}\"", site, short_url));
                    }
                    Some((site, url))
                } else {
                    let site = browser::extract_site(&window_title);
                    if dev_mode {
                        dev_log(&app, "URL", "error", &format!(
                            "FAILED to get URL from \"{}\" — fallback site=\"{}\"", app_name, site
                        ));
                    }
                    Some((site, window_title.clone()))
                }
            } else {
                None
            };

            // Skip module reactions for first 10s so AI greeting shows first
            if now_ms - startup_time < 10_000 {
                continue;
            }

            // Global cooldown check
            if now_ms - last_reaction_time < reaction_cooldown_ms {
                if dev_mode {
                    let remaining = (reaction_cooldown_ms - (now_ms - last_reaction_time)) / 1000;
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
                let result = reaction_engine.evaluate_detailed(&browser_event, &pairs).await;
                if dev_mode {
                    match &result {
                        super::reaction_engine::EvalResult::NoMatch => {
                            dev_log(&app, "MISS", "skip", &format!("no module matches site=\"{}\"", site));
                        }
                        super::reaction_engine::EvalResult::OnCooldown { module_id, reaction_id } => {
                            dev_log(&app, "COOLDOWN", "skip", &format!("{}::{} (on cooldown)", module_id, reaction_id));
                        }
                        _ => {}
                    }
                }
                match result {
                    super::reaction_engine::EvalResult::Fired(f) => Some(f),
                    _ => None,
                }
            } else {
                let loader = module_loader.read().await;
                let active = loader.get_active_reactions();
                let pairs: Vec<_> = active.iter().map(|(m, r)| (*m, *r)).collect();
                let result = reaction_engine.evaluate_detailed(&event, &pairs).await;
                if dev_mode {
                    match &result {
                        super::reaction_engine::EvalResult::NoMatch => {
                            dev_log(&app, "MISS", "skip", &format!("no module matches app=\"{}\"", app_name));
                        }
                        super::reaction_engine::EvalResult::OnCooldown { module_id, reaction_id } => {
                            dev_log(&app, "COOLDOWN", "skip", &format!("{}::{} (on cooldown)", module_id, reaction_id));
                        }
                        _ => {}
                    }
                }
                match result {
                    super::reaction_engine::EvalResult::Fired(f) => Some(f),
                    _ => None,
                }
            };

            if let Some(fired) = fired {
                last_reaction_time = now_ms;
                if dev_mode {
                    dev_log(&app, "FIRE", "reaction", &format!(
                        "module=\"{}\" id=\"{}\"", fired.module_id, fired.reaction_id
                    ));
                }
                let _ = app.emit("module-reaction", serde_json::json!({
                    "module_id": fired.module_id,
                    "message": fired.message,
                    "priority": fired.priority,
                }));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cat_status_for_coding() {
        assert_eq!(cat_status_for_app("code"), "🐾 看你写代码...");
        assert_eq!(cat_status_for_app("intellij idea"), "🐾 看你写代码...");
    }

    #[test]
    fn test_cat_status_for_terminal() {
        assert_eq!(cat_status_for_app("terminal"), "💻 盯着终端看...");
        assert_eq!(cat_status_for_app("iterm2"), "💻 盯着终端看...");
    }

    #[test]
    fn test_cat_status_for_zoom() {
        assert_eq!(cat_status_for_app("zoom"), "🤫 安静等你开完会...");
    }

    #[test]
    fn test_cat_status_for_browser() {
        assert_eq!(cat_status_for_app("google chrome"), "🌐 看你上网冲浪...");
    }

    #[test]
    fn test_cat_status_default() {
        assert_eq!(cat_status_for_app("finder"), "😺 好奇地看着...");
    }
}
