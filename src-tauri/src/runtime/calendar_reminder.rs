use tauri::Emitter;

const CHECK_INTERVAL_SECS: u64 = 300; // 5 minutes
const REMIND_WITHIN_MINUTES: i32 = 10;

const MEETING_MESSAGES: &[&str] = &[
    "Meeting in {} min! 准备好了吗？🐱",
    "开会啦！{}分钟后开始～别迟到！",
    "{} min to meeting! Time to find your mute button 🎤",
    "会议快开始了！先喝口水再进去～💧",
    "{} min! 该开会了！快切到Zoom/Chime！🏃",
    "Meeting alert! {}分钟后要开始了～站起来伸个懒腰再进去！",
    "{} min to meeting... have you prepared? 🤔",
    "快了快了！{}分钟后开会！趁现在上个厕所！🚻",
];

pub struct CalendarReminder {
    enabled: bool,
    alias: String,
    last_check: u64,
    last_reminded: String,
}

impl CalendarReminder {
    pub fn new() -> Self {
        Self {
            enabled: false,
            alias: String::new(),
            last_check: 0,
            last_reminded: String::new(),
        }
    }

    pub fn update_config(&mut self, enabled: bool, alias: String) {
        self.enabled = enabled;
        self.alias = alias;
    }

    pub fn tick(&mut self, app: &tauri::AppHandle, now_secs: u64) {
        if !self.enabled || self.alias.is_empty() {
            return;
        }

        if now_secs - self.last_check < CHECK_INTERVAL_SECS {
            return;
        }
        self.last_check = now_secs;

        let events = match super::mcp_runner::call_mcp(
            "amazon-internal",
            &["get-calendar", "--alias", &self.alias, "--hours", "1"],
        ) {
            Ok(val) => val,
            Err(_) => return,
        };

        let events_arr = match events.as_array() {
            Some(arr) => arr,
            None => return,
        };

        let now_hhmm = chrono::Local::now().format("%H:%M").to_string();
        let now_minutes = hhmm_to_minutes(&now_hhmm);

        for event in events_arr {
            let start = match event["start"].as_str() {
                Some(s) => s,
                None => continue,
            };

            let start_minutes = hhmm_to_minutes(start);
            let diff = start_minutes - now_minutes;

            if diff > 0 && diff <= REMIND_WITHIN_MINUTES && start != self.last_reminded {
                self.last_reminded = start.to_string();

                let idx = (now_secs as usize) % MEETING_MESSAGES.len();
                let message = MEETING_MESSAGES[idx].replace("{}", &diff.to_string());

                let _ = app.emit("module-reaction", serde_json::json!({
                    "module_id": "calendar",
                    "message": message,
                    "priority": 8,
                }));
                return;
            }
        }
    }
}

fn hhmm_to_minutes(hhmm: &str) -> i32 {
    let parts: Vec<&str> = hhmm.split(':').collect();
    if parts.len() != 2 {
        return 0;
    }
    let hours: i32 = parts[0].parse().unwrap_or(0);
    let mins: i32 = parts[1].parse().unwrap_or(0);
    hours * 60 + mins
}
