use std::process::Command;

pub fn get_context() -> String {
    let window = get_active_window();
    let category = categorize_activity(&window);
    let typing = is_typing();
    if typing {
        format!("{} [typing]", category)
    } else {
        category
    }
}

fn is_typing() -> bool {
    let output = Command::new("ioreg")
        .args(["-c", "IOHIDSystem"])
        .output();
    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("HIDIdleTime") {
                    if let Some(val) = line.split_whitespace().last() {
                        if let Ok(ns) = val.parse::<u64>() {
                            return ns / 1_000_000_000 < 2;
                        }
                    }
                }
            }
            false
        }
        Err(_) => false,
    }
}

pub fn categorize_activity(raw: &str) -> String {
    let lower = raw.to_lowercase();

    if ["code", "intellij", "xcode", "vim", "nvim", "terminal", "iterm", "warp", "cursor", "webstorm", "pycharm", "clion"]
        .iter().any(|app| lower.contains(app)) {
        return format!("Coding ({})", raw);
    }

    if ["twitter", "reddit", "instagram", "tiktok", "youtube", "bilibili", "weibo", "douyin", "xiaohongshu"]
        .iter().any(|site| lower.contains(site)) {
        return format!("Social media / slacking off ({})", raw);
    }

    if ["slack", "teams", "discord", "wechat", "telegram", "messages", "zoom", "chime"]
        .iter().any(|app| lower.contains(app)) {
        return format!("Communication ({})", raw);
    }

    if ["chrome", "safari", "firefox", "edge", "arc", "brave"]
        .iter().any(|browser| lower.contains(browser)) {
        return format!("Web browsing ({})", raw);
    }

    if ["notion", "confluence", "quip", "preview", "pdf"]
        .iter().any(|app| lower.contains(app)) {
        return format!("Reading/Documentation ({})", raw);
    }

    format!("Other ({})", raw)
}

#[tauri::command]
pub fn get_active_window() -> String {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events"
            set frontApp to name of first application process whose frontmost is true
            set windowTitle to ""
            try
                tell process frontApp
                    set windowTitle to name of front window
                end tell
            end try
            return frontApp & " - " & windowTitle
        end tell"#)
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "Unknown".to_string(),
    }
}
