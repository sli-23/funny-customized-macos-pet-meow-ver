#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacOsPlatform as NativePlatform;

use std::collections::HashMap;

pub trait Platform: Send + Sync {
    fn get_active_window(&self) -> String;
    fn is_typing(&self) -> bool;
    fn get_battery_info(&self) -> Option<String>;
    fn get_idle_seconds(&self) -> Option<u64>;
    fn get_browser_url(&self, app_name: &str) -> Option<String>;
    fn is_process_running(&self, name: &str) -> bool;
    fn run_applescript(&self, script: &str) -> Option<String>;
    fn check_permissions_status(&self) -> HashMap<String, bool>;
    fn open_system_settings(&self, url: &str);
}

pub fn native() -> NativePlatform {
    NativePlatform
}
