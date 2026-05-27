use crate::util::now_secs;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThemeEntry {
    theme: String,
    last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ThemeTracker {
    themes: Vec<ThemeEntry>,
}

struct ThemeRule {
    key: &'static str,
    keywords: &'static [&'static str],
}

const THEME_RULES: &[ThemeRule] = &[
    ThemeRule { key: "go_home", keywords: &["下班", "回家", "go home", "休息"] },
    ThemeRule { key: "drink_water", keywords: &["喝水", "补水"] },
    ThemeRule { key: "sleep", keywords: &["睡觉", "sleep", "早点休息", "晚安"] },
    ThemeRule { key: "posture", keywords: &["坐直", "驼背", "posture"] },
    ThemeRule { key: "time_comment", keywords: &["早上好", "good morning", "下午了"] },
];

fn themes_path() -> PathBuf {
    crate::paths::memory_dir().join("recent_themes.json")
}

fn load_themes() -> ThemeTracker {
    fs::read_to_string(themes_path())
        .ok()
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_default()
}

fn save_themes(tracker: &ThemeTracker) {
    let _ = fs::write(themes_path(), serde_json::to_string(tracker).unwrap_or_default());
}

pub fn classify_theme(message: &str) -> Option<&'static str> {
    let lower = message.to_lowercase();
    for rule in THEME_RULES {
        for keyword in rule.keywords {
            if lower.contains(keyword) {
                return Some(rule.key);
            }
        }
    }
    None
}

pub fn is_theme_blocked(message: &str) -> bool {
    let theme = match classify_theme(message) {
        Some(t) => t,
        None => return false,
    };
    let tracker = load_themes();
    let now = now_secs();
    let hour = chrono::Local::now().hour();

    if theme == "go_home" && hour >= 17 {
        return tracker
            .themes
            .iter()
            .any(|t| t.theme == "go_home" && now - t.last_used < 28800);
    }

    tracker
        .themes
        .iter()
        .any(|t| t.theme == theme && now - t.last_used < 3600)
}

pub fn record_theme(message: &str) {
    let theme = match classify_theme(message) {
        Some(t) => t,
        None => return,
    };
    let mut tracker = load_themes();
    let now = now_secs();

    if let Some(entry) = tracker.themes.iter_mut().find(|t| t.theme == theme) {
        entry.last_used = now;
    } else {
        tracker.themes.push(ThemeEntry {
            theme: theme.to_string(),
            last_used: now,
        });
    }

    tracker.themes.retain(|t| now - t.last_used < 7200);
    save_themes(&tracker);
}

pub fn get_blocked_themes_for_prompt() -> String {
    let tracker = load_themes();
    let now = now_secs();
    let blocked: Vec<&str> = tracker
        .themes
        .iter()
        .filter(|t| now - t.last_used < 3600)
        .map(|t| t.theme.as_str())
        .collect();

    if blocked.is_empty() {
        return String::new();
    }

    let descriptions: Vec<&str> = blocked
        .iter()
        .map(|t| match *t {
            "go_home" => "going home/下班/休息",
            "drink_water" => "drinking water/喝水",
            "sleep" => "sleeping/睡觉",
            "posture" => "posture/坐直",
            "time_comment" => "time-of-day greetings",
            _ => "",
        })
        .filter(|s| !s.is_empty())
        .collect();

    format!(
        "\nDO NOT talk about: {}. Say something completely different.",
        descriptions.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_theme_go_home() {
        assert_eq!(classify_theme("该下班了"), Some("go_home"));
        assert_eq!(classify_theme("go home now"), Some("go_home"));
    }

    #[test]
    fn test_classify_theme_water() {
        assert_eq!(classify_theme("记得喝水"), Some("drink_water"));
    }

    #[test]
    fn test_classify_theme_sleep() {
        assert_eq!(classify_theme("早点sleep吧"), Some("sleep"));
        assert_eq!(classify_theme("晚安~"), Some("sleep"));
    }

    #[test]
    fn test_classify_theme_posture() {
        assert_eq!(classify_theme("坐直一点"), Some("posture"));
        assert_eq!(classify_theme("fix your posture"), Some("posture"));
    }

    #[test]
    fn test_classify_theme_none() {
        assert_eq!(classify_theme("nice code commit!"), None);
        assert_eq!(classify_theme("在写什么呢"), None);
    }

    #[test]
    fn test_classify_theme_case_insensitive() {
        assert_eq!(classify_theme("GO HOME"), Some("go_home"));
        assert_eq!(classify_theme("Good Morning!"), Some("time_comment"));
    }

    #[test]
    fn test_water_does_not_false_positive_on_waterfall() {
        // "water" is no longer a keyword — we use "喝水" and "补水"
        // so "waterfall" should NOT match drink_water
        assert_eq!(classify_theme("waterfall architecture"), None);
    }
}
