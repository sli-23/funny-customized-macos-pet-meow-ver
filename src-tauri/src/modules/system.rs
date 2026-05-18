use std::process::Command;

pub fn get_context() -> String {
    let mut parts = Vec::new();

    // Battery
    if let Ok(out) = Command::new("pmset").args(["-g", "batt"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains('%') {
                let trimmed = line.trim();
                if let Some(pct_pos) = trimmed.find('%') {
                    let start = trimmed[..pct_pos]
                        .rfind(|c: char| !c.is_ascii_digit())
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let pct = &trimmed[start..=pct_pos];
                    let charging = if trimmed.contains("charging") && !trimmed.contains("not charging") {
                        " (charging)"
                    } else if trimmed.contains("AC Power") || trimmed.contains("charged") {
                        " (plugged in)"
                    } else {
                        " (battery)"
                    };
                    parts.push(format!("Battery: {}{}", pct, charging));
                }
                break;
            }
        }
    }

    // Idle time
    if let Ok(out) = Command::new("ioreg").args(["-c", "IOHIDSystem"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("HIDIdleTime") {
                if let Some(val) = line.split_whitespace().last() {
                    if let Ok(ns) = val.parse::<u64>() {
                        let secs = ns / 1_000_000_000;
                        if secs > 60 {
                            parts.push(format!("Idle: {}m", secs / 60));
                        }
                    }
                }
                break;
            }
        }
    }

    if parts.is_empty() {
        "System: normal".to_string()
    } else {
        parts.join(", ")
    }
}
