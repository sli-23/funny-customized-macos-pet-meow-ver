pub mod activity;
pub mod performance;
pub mod quotes;
pub mod spotify;
pub mod status;
pub mod system;
pub mod user_profile;
pub mod weather;
pub mod zoom;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleContext {
    pub activity: String,
    pub weather: String,
    pub spotify: String,
    pub system: String,
    pub performance: String,
    pub zoom: String,
    pub quotes: String,
    pub user_profile: String,
}

/// Lightweight — activity + spotify + user profile (no network, no heavy system calls)
#[tauri::command]
pub fn get_activity_context() -> String {
    format_activity_context(
        &activity::get_context(),
        &spotify::get_context(),
        &user_profile::get_context(),
    )
}

pub(crate) fn format_activity_context(activity: &str, spotify: &str, profile: &str) -> String {
    let mut parts = vec![activity.to_string()];
    if !spotify.contains("not running") {
        parts.push(spotify.to_string());
    }
    if !profile.is_empty() {
        parts.push(profile.to_string());
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_activity_context_excludes_spotify_not_running() {
        let s = format_activity_context("VS Code", "Spotify not running", "");
        assert!(!s.contains("not running"), "got: {}", s);
    }

    #[test]
    fn test_format_activity_context_includes_spotify_when_playing() {
        let s = format_activity_context("VS Code", "Playing: Lo-fi Beats", "");
        assert!(s.contains("Playing: Lo-fi Beats"), "got: {}", s);
    }

    #[test]
    fn test_format_activity_context_excludes_empty_profile() {
        let s = format_activity_context("VS Code", "Spotify not running", "");
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_format_activity_context_includes_nonempty_profile() {
        let s = format_activity_context("VS Code", "Spotify not running", "Name: Tom");
        assert!(s.contains("Name: Tom"), "got: {}", s);
    }

    #[test]
    fn test_format_activity_context_joins_with_newlines() {
        let s = format_activity_context("VS Code", "Playing: Jazz", "Name: Tom");
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 3);
    }
}

/// Heavy — all modules including weather (curl), performance (top), etc.
#[tauri::command]
pub fn get_all_context() -> ModuleContext {
    ModuleContext {
        activity: activity::get_context(),
        weather: weather::get_context(),
        spotify: spotify::get_context(),
        system: system::get_context(),
        performance: performance::get_context(),
        zoom: zoom::get_context(),
        quotes: quotes::get_context(),
        user_profile: user_profile::get_context(),
    }
}
