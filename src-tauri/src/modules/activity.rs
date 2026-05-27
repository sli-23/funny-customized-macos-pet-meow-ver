use crate::platform::{self, Platform};

pub fn get_context() -> String {
    let plat = platform::native();
    let window = plat.get_active_window();
    let parts: Vec<&str> = window.splitn(2, " - ").collect();
    let app_name = parts.first().unwrap_or(&"");
    let is_self = app_name.eq_ignore_ascii_case("claude-meow-pet")
        || app_name.eq_ignore_ascii_case("ClaudeMeow");
    let filtered = if is_self {
        "Unknown".to_string()
    } else {
        window
    };
    let category = categorize_activity(&filtered);
    let typing = plat.is_typing();
    if typing {
        format!("{} [typing]", category)
    } else {
        category
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

pub fn is_typing() -> bool {
    let plat = platform::native();
    plat.is_typing()
}

#[tauri::command]
pub fn get_active_window() -> String {
    let plat = platform::native();
    plat.get_active_window()
}
