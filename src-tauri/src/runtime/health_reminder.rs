use tauri::Emitter;

const HEALTH_MESSAGES: &[&str] = &[
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
    "手腕转一转！预防腱鞘炎！🖐",
    "Blink blink! 眨眨眼睛防干眼 👁",
    "屏幕亮度调低点～护眼！🌙",
    "吃点水果补充维C！🍊",
    "坐久了腰疼！起来扭扭腰！💃",
    "别憋尿！去一趟洗手间！🚻",
    "下巴收一收！不要前伸！🐢",
    "脚放平！不要翘脚尖！🦶",
    "零食少吃点！正餐好好吃！🍱",
    "Your spine called. It wants a divorce. 坐直！🦴",
    "Hydrate or diedrate~ 喝水！💀💧",
    "Legs crossed = spine cursed. Uncross! 🦵❌",
    "Your eyes are not monitors. Look away! 👀🌳",
    "Stand up! Your chair misses being empty 🪑",
    "Wrists say: rotate me or regret me 🔄🖐",
    "You've been a statue for too long. Move! 🗿➡️🏃",
    "Shoulders up by your ears again huh? Drop em! 😤",
    "Your future self says thanks for stretching now 🧘",
    "Water break! Your kidneys are filing a complaint 💧⚖️",
];

pub struct HealthReminder {
    last_fired: u64,
    enabled: bool,
    interval_secs: u64,
    quiet_start: u8,
    quiet_end: u8,
}

impl HealthReminder {
    pub fn new() -> Self {
        Self {
            last_fired: 0,
            enabled: true,
            interval_secs: 900,
            quiet_start: 23,
            quiet_end: 8,
        }
    }

    pub fn update_config(&mut self, enabled: bool, interval_minutes: u32, quiet_start: u8, quiet_end: u8) {
        self.enabled = enabled;
        self.interval_secs = (interval_minutes as u64) * 60;
        self.quiet_start = quiet_start;
        self.quiet_end = quiet_end;
    }

    pub fn tick(&mut self, app: &tauri::AppHandle, now_secs: u64) {
        if !self.enabled {
            return;
        }

        if self.is_quiet_hours() {
            return;
        }

        if now_secs - self.last_fired >= self.interval_secs {
            self.last_fired = now_secs;
            let idx = (now_secs as usize) % HEALTH_MESSAGES.len();
            let _ = app.emit("module-reaction", serde_json::json!({
                "module_id": "health",
                "message": HEALTH_MESSAGES[idx],
                "priority": 9,
            }));
        }
    }

    fn is_quiet_hours(&self) -> bool {
        let hour = chrono::Local::now().hour() as u8;
        if self.quiet_start <= self.quiet_end {
            hour >= self.quiet_start && hour < self.quiet_end
        } else {
            hour >= self.quiet_start || hour < self.quiet_end
        }
    }
}

use chrono::Timelike;
