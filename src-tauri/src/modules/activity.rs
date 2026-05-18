use std::process::Command;

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
