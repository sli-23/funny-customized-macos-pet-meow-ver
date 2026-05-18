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
    let mut parts = vec![activity::get_context()];
    let spot = spotify::get_context();
    if !spot.contains("not running") {
        parts.push(spot);
    }
    let profile = user_profile::get_context();
    if !profile.is_empty() {
        parts.push(profile);
    }
    parts.join("\n")
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
