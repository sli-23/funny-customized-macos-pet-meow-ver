use std::collections::HashMap;
use tauri::Emitter;

const SCREEN_TIME_THRESHOLD_SECS: u64 = 14400; // 4 hours
const SCREEN_TIME_ALERT_GAP_SECS: u64 = 1800; // 30 min between alerts

pub struct ScreenTimeTracker {
    app_times: HashMap<String, u64>,
    current_app: String,
    current_start: u64,
    last_alert: u64,
}

impl ScreenTimeTracker {
    pub fn new() -> Self {
        Self {
            app_times: HashMap::new(),
            current_app: String::new(),
            current_start: 0,
            last_alert: 0,
        }
    }

    pub fn on_app_switch(&mut self, app: &tauri::AppHandle, app_name: &str, now_secs: u64, dev_mode: bool) {
        if !self.current_app.is_empty() && self.current_start > 0 {
            let elapsed = now_secs - self.current_start;
            *self.app_times.entry(self.current_app.clone()).or_insert(0) += elapsed;
        }
        self.current_app = app_name.to_string();
        self.current_start = now_secs;

        let is_browser_app = ["Google Chrome", "Safari", "Firefox", "Microsoft Edge", "Arc", "Brave"]
            .iter().any(|b| app_name.contains(b));
        let total_secs = self.app_times.get(app_name).copied().unwrap_or(0);

        if !is_browser_app
            && total_secs > SCREEN_TIME_THRESHOLD_SECS
            && now_secs - self.last_alert > SCREEN_TIME_ALERT_GAP_SECS
        {
            self.last_alert = now_secs;
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let messages = [
                format!("你今天在{}上已经{}小时{}分钟了！休息一下吧！🐱", app_name, hours, mins),
                format!("{}用了{}小时了！眼睛不累吗？👀", app_name, hours),
                format!("{}打开太久了！站起来走走！🚶", app_name),
                format!("已经在{}上{}h{}m了...该休息了！💤", app_name, hours, mins),
            ];
            let idx = (now_secs as usize) % messages.len();
            let _ = app.emit("module-reaction", serde_json::json!({
                "module_id": "screen-time",
                "message": messages[idx],
                "priority": 8,
            }));
            if dev_mode {
                let _ = app.emit("dev-log", serde_json::json!({
                    "tag": "TIME", "tag_class": "error",
                    "message": format!("{}h{}m on \"{}\"", hours, mins, app_name),
                }));
            }
        }
    }
}
