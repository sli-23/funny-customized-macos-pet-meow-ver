use chrono::Timelike;

use crate::util::now_secs;

pub(crate) fn is_coding_app(app: &str) -> bool {
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
        let now = now_secs();
        Self {
            current_app: String::new(),
            app_start_time: now,
            app_switch_times: Vec::new(),
            coding_start: None,
            last_break: now,
            rage_count: 0,
            chat_count: 0,
            last_rage: 0,
            last_chat: 0,
        }
    }

    pub fn on_app_switch(&mut self, app: &str) {
        self.on_app_switch_at(app, now_secs());
    }

    pub(crate) fn on_app_switch_at(&mut self, app: &str, now: u64) {
        self.app_switch_times.push(now);
        self.app_switch_times.retain(|t| now.saturating_sub(*t) < 3600);

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
        self.on_rage_at(now_secs());
    }

    pub(crate) fn on_rage_at(&mut self, now: u64) {
        self.rage_count += 1;
        self.last_rage = now;
    }

    pub fn on_chat(&mut self) {
        self.on_chat_at(now_secs());
    }

    pub(crate) fn on_chat_at(&mut self, now: u64) {
        self.chat_count += 1;
        self.last_chat = now;
    }

    pub fn get_context_string(&self) -> String {
        self.get_context_string_at(now_secs())
    }

    pub(crate) fn get_context_string_at(&self, now: u64) -> String {
        let mut parts = Vec::new();

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

        if let Some(start) = self.coding_start {
            let mins = now.saturating_sub(start) / 60;
            if mins >= 30 {
                parts.push(format!("[Streak: coding for {} min straight without break — suggest stretching/rest]", mins));
            }
        }

        let recent_switches = self.app_switch_times.iter()
            .filter(|t| now.saturating_sub(**t) < 600)
            .count();
        if recent_switches >= 10 {
            parts.push("[Pattern: user switching apps very rapidly — seems restless or distracted, tease gently]".to_string());
        }

        if self.last_rage > 0 && now.saturating_sub(self.last_rage) < 600 {
            parts.push("[Mood: cautious — user raged recently, approach gently, be soft and apologetic]".to_string());
        } else if self.rage_count >= 3 {
            parts.push(format!("[Mood: wary — user has raged {} times today, be careful]", self.rage_count));
        }

        if self.chat_count >= 5 {
            parts.push("[Mood: extra affectionate — user chatted a lot today, be clingy and loving]".to_string());
        } else if self.chat_count >= 2 && now.saturating_sub(self.last_chat) < 300 {
            parts.push("[Mood: happy — user just chatted, feel appreciated and playful]".to_string());
        }

        let app_mins = now.saturating_sub(self.app_start_time) / 60;
        if app_mins >= 20 && !self.current_app.is_empty() {
            parts.push(format!("[Duration: on {} for {} min]", self.current_app, app_mins));
        }

        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_700_000_000; // fixed epoch for tests

    // ── Checkpoint 3: is_coding_app ──────────────────────────────────────────

    #[test]
    fn test_is_coding_app_matches_vscode() {
        assert!(is_coding_app("Visual Studio Code"));
    }

    #[test]
    fn test_is_coding_app_matches_cursor() {
        assert!(is_coding_app("Cursor"));
    }

    #[test]
    fn test_is_coding_app_matches_xcode() {
        assert!(is_coding_app("Xcode"));
    }

    #[test]
    fn test_is_coding_app_matches_terminal() {
        assert!(is_coding_app("Terminal"));
    }

    #[test]
    fn test_is_coding_app_false_for_safari() {
        assert!(!is_coding_app("Safari"));
    }

    #[test]
    fn test_is_coding_app_false_for_slack() {
        assert!(!is_coding_app("Slack"));
    }

    #[test]
    fn test_is_coding_app_case_insensitive() {
        assert!(is_coding_app("INTELLIJ IDEA"));
    }

    // ── Checkpoint 3: on_app_switch_at ───────────────────────────────────────

    #[test]
    fn test_on_app_switch_updates_current_app() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Safari", T0);
        assert_eq!(ctx.current_app, "Safari");
    }

    #[test]
    fn test_on_app_switch_records_switch_time() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Safari", T0);
        assert_eq!(ctx.app_switch_times.len(), 1);
    }

    #[test]
    fn test_on_app_switch_starts_coding_session_for_ide() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Visual Studio Code", T0);
        assert!(ctx.coding_start.is_some());
    }

    #[test]
    fn test_on_app_switch_ends_coding_session_for_non_ide() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Visual Studio Code", T0);
        ctx.on_app_switch_at("Safari", T0 + 60);
        assert!(ctx.coding_start.is_none());
    }

    #[test]
    fn test_on_app_switch_retains_only_last_hour_of_switches() {
        let mut ctx = ContextEngine::new();
        // add a switch that's more than 1 hour old
        ctx.on_app_switch_at("Safari", T0);
        // add a recent switch
        ctx.on_app_switch_at("Slack", T0 + 3601);
        // the old one should be pruned
        assert_eq!(ctx.app_switch_times.len(), 1);
    }

    // ── Checkpoint 3: on_rage_at / on_chat_at counters ───────────────────────

    #[test]
    fn test_on_rage_increments_counter() {
        let mut ctx = ContextEngine::new();
        ctx.on_rage_at(T0);
        ctx.on_rage_at(T0 + 1);
        assert_eq!(ctx.rage_count, 2);
    }

    #[test]
    fn test_on_rage_sets_last_rage_timestamp() {
        let mut ctx = ContextEngine::new();
        ctx.on_rage_at(T0 + 42);
        assert_eq!(ctx.last_rage, T0 + 42);
    }

    #[test]
    fn test_on_chat_increments_counter() {
        let mut ctx = ContextEngine::new();
        ctx.on_chat_at(T0);
        ctx.on_chat_at(T0 + 1);
        assert_eq!(ctx.chat_count, 2);
    }

    // ── Checkpoint 4: get_context_string_at (time-controlled) ────────────────

    #[test]
    fn test_get_context_string_mentions_duration_after_20min() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Safari", T0);
        let s = ctx.get_context_string_at(T0 + 21 * 60);
        assert!(s.contains("[Duration: on Safari for 21 min]"), "got: {}", s);
    }

    #[test]
    fn test_get_context_string_no_duration_under_20min() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Safari", T0);
        let s = ctx.get_context_string_at(T0 + 10 * 60);
        assert!(!s.contains("[Duration:"), "got: {}", s);
    }

    #[test]
    fn test_get_context_string_coding_streak_after_30min() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Visual Studio Code", T0);
        let s = ctx.get_context_string_at(T0 + 31 * 60);
        assert!(s.contains("[Streak:"), "got: {}", s);
        assert!(s.contains("31 min"), "got: {}", s);
    }

    #[test]
    fn test_get_context_string_no_streak_under_30min() {
        let mut ctx = ContextEngine::new();
        ctx.on_app_switch_at("Visual Studio Code", T0);
        let s = ctx.get_context_string_at(T0 + 29 * 60);
        assert!(!s.contains("[Streak:"), "got: {}", s);
    }

    #[test]
    fn test_get_context_string_frequent_switching_flagged() {
        let mut ctx = ContextEngine::new();
        for i in 0..10 {
            ctx.on_app_switch_at("App", T0 + i * 30);
        }
        let s = ctx.get_context_string_at(T0 + 10 * 30);
        assert!(s.contains("[Pattern:"), "got: {}", s);
    }

    #[test]
    fn test_get_context_string_recent_rage_sets_cautious_mood() {
        let mut ctx = ContextEngine::new();
        ctx.on_rage_at(T0);
        let s = ctx.get_context_string_at(T0 + 60); // 1 min later, within 600s window
        assert!(s.contains("[Mood: cautious"), "got: {}", s);
    }

    #[test]
    fn test_get_context_string_old_rage_does_not_set_cautious() {
        let mut ctx = ContextEngine::new();
        ctx.on_rage_at(T0);
        let s = ctx.get_context_string_at(T0 + 700); // outside 600s window
        assert!(!s.contains("[Mood: cautious"), "got: {}", s);
    }

    #[test]
    fn test_get_context_string_high_chat_count_sets_engaged_mood() {
        let mut ctx = ContextEngine::new();
        for i in 0..5 { ctx.on_chat_at(T0 + i); }
        let s = ctx.get_context_string_at(T0 + 10);
        assert!(s.contains("[Mood: extra affectionate"), "got: {}", s);
    }
}
