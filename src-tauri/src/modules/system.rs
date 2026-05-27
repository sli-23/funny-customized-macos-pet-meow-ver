use crate::platform::{self, Platform};

pub fn get_context() -> String {
    let plat = platform::native();
    let mut parts = Vec::new();

    if let Some(battery) = plat.get_battery_info() {
        parts.push(battery);
    }

    if let Some(secs) = plat.get_idle_seconds() {
        if secs > 60 {
            parts.push(format!("Idle: {}m", secs / 60));
        }
    }

    if parts.is_empty() {
        "System: normal".to_string()
    } else {
        parts.join(", ")
    }
}
