use tauri::Emitter;

const CHECK_INTERVAL_SECS: u64 = 120; // 2 minutes
const REMIND_WITHIN_MINUTES: i32 = 10;
const DAILY_FETCH_HOURS: u64 = 12; // fetch full day events (12 hours ahead)

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
    cached_events: Vec<serde_json::Value>,
    last_daily_fetch: u64,
}

impl CalendarReminder {
    pub fn new() -> Self {
        Self {
            enabled: false,
            alias: String::new(),
            last_check: 0,
            last_reminded: String::new(),
            cached_events: Vec::new(),
            last_daily_fetch: 0,
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

        // Daily fetch: refresh full day calendar every morning (or on first run)
        let hour = chrono::Local::now().format("%H").to_string().parse::<u64>().unwrap_or(0);
        if self.last_daily_fetch == 0 || (hour >= 8 && now_secs - self.last_daily_fetch > 3600 * 8) {
            self.fetch_daily_events();
            self.last_daily_fetch = now_secs;
        }

        // Check cached events every 2 minutes
        if now_secs - self.last_check < CHECK_INTERVAL_SECS {
            return;
        }
        self.last_check = now_secs;

        // If no cached events, try a quick fetch
        if self.cached_events.is_empty() {
            self.fetch_daily_events();
        }

        let now_hhmm = chrono::Local::now().format("%H:%M").to_string();
        let now_minutes = hhmm_to_minutes(&now_hhmm);

        for event in &self.cached_events {
            let start = match event["start"].as_str() {
                Some(s) => s,
                None => continue,
            };

            let start_minutes = hhmm_to_minutes(start);
            let diff = start_minutes - now_minutes;

            // Remind if meeting is within 10 min OR just started (within last 5 min)
            if diff >= -5 && diff <= REMIND_WITHIN_MINUTES && start != self.last_reminded {
                self.last_reminded = start.to_string();

                let idx = (now_secs as usize) % MEETING_MESSAGES.len();
                let message = if diff <= 0 {
                    "Meeting已经开始了！快进去！🏃".to_string()
                } else {
                    MEETING_MESSAGES[idx].replace("{}", &diff.to_string())
                };

                let _ = app.emit("module-reaction", serde_json::json!({
                    "module_id": "calendar",
                    "message": message,
                    "priority": 8,
                }));
                return;
            }
        }
    }

    fn fetch_daily_events(&mut self) {
        let hours_str = DAILY_FETCH_HOURS.to_string();
        match super::mcp_runner::call_mcp(
            "amazon-internal",
            &["get-calendar", "--alias", &self.alias, "--hours", &hours_str],
        ) {
            Ok(val) => {
                if let Some(arr) = val.as_array() {
                    self.cached_events = arr.clone();
                }
            }
            Err(_) => {}
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

