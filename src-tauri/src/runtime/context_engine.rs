use chrono::Timelike;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn is_coding_app(app: &str) -> bool {
    let lower = app.to_lowercase();
    ["code", "intellij", "xcode", "vim", "nvim", "terminal", "iterm",
     "warp", "cursor", "webstorm", "pycharm", "clion", "kiro"]
        .iter().any(|a| lower.contains(a))
}

pub struct ContextEngine {
    current_app: String,
    app_start_time: u64,
    app_switch_times: Vec<u64>,
    coding_start: Option<u64>,
    last_break: u64,
    rage_count: u32,
    chat_count: u32,
    last_rage: u64,
    last_chat: u64,
}

impl ContextEngine {
    pub fn new() -> Self {
        Self {
            current_app: String::new(),
            app_start_time: now_secs(),
            app_switch_times: Vec::new(),
            coding_start: None,
            last_break: now_secs(),
            rage_count: 0,
            chat_count: 0,
            last_rage: 0,
            last_chat: 0,
        }
    }

    pub fn on_app_switch(&mut self, app: &str) {
        let now = now_secs();

        self.app_switch_times.push(now);
        self.app_switch_times.retain(|t| now - t < 3600);

        if is_coding_app(app) {
            if self.coding_start.is_none() {
                self.coding_start = Some(now);
            }
        } else {
            if self.coding_start.is_some() {
                self.last_break = now;
            }
            self.coding_start = None;
        }

        self.current_app = app.to_string();
        self.app_start_time = now;
    }

    pub fn on_rage(&mut self) {
        self.rage_count += 1;
        self.last_rage = now_secs();
    }

    pub fn on_chat(&mut self) {
        self.chat_count += 1;
        self.last_chat = now_secs();
    }

    pub fn get_context_string(&self) -> String {
        let now = now_secs();
        let mut parts = Vec::new();

        // Time personality
        let hour = chrono::Local::now().hour();
        let time_desc = match hour {
            6..=8 => "early morning — be energetic and cheerful, greet them",
            9..=11 => "mid-morning — productive energy, encourage them",
            12..=13 => "lunch time — remind about food, be hungry together",
            14..=16 => "afternoon — slightly tired, gentle encouragement",
            17..=18 => "end of work day — suggest going home, winding down",
            19..=21 => "evening — cozy, relaxed, sleepy vibes starting",
            22..=23 => "late night — very sleepy, yawning, nag them to sleep",
            0..=5 => "deep night — extremely sleepy, concerned they're still awake",
            _ => "daytime",
        };
        parts.push(format!("[Time: {}]", time_desc));

        // Coding streak
        if let Some(start) = self.coding_start {
            let mins = (now - start) / 60;
            if mins >= 30 {
                parts.push(format!("[Streak: coding for {} min straight without break — suggest stretching/rest]", mins));
            }
        }

        // App switching frequency (restlessness)
        let recent_switches = self.app_switch_times.iter()
            .filter(|t| now - *t < 600)
            .count();
        if recent_switches >= 10 {
            parts.push("[Pattern: user switching apps very rapidly — seems restless or distracted, tease gently]".to_string());
        }

        // Mood from rage
        if self.last_rage > 0 && now - self.last_rage < 600 {
            parts.push("[Mood: cautious — user raged recently, approach gently, be soft and apologetic]".to_string());
        } else if self.rage_count >= 3 {
            parts.push(format!("[Mood: wary — user has raged {} times today, be careful]", self.rage_count));
        }

        // Mood from chat
        if self.chat_count >= 5 {
            parts.push("[Mood: extra affectionate — user chatted a lot today, be clingy and loving]".to_string());
        } else if self.chat_count >= 2 && now - self.last_chat < 300 {
            parts.push("[Mood: happy — user just chatted, feel appreciated and playful]".to_string());
        }

        // Current app duration
        let app_mins = (now - self.app_start_time) / 60;
        if app_mins >= 20 && !self.current_app.is_empty() {
            parts.push(format!("[Duration: on {} for {} min]", self.current_app, app_mins));
        }

        parts.join("\n")
    }
}
