use std::process::Command;

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
