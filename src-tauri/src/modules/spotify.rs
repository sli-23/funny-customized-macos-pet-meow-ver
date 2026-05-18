use std::process::Command;

pub fn get_context() -> String {
    let running = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to (name of processes) contains "Spotify""#)
        .output();

    match running {
        Ok(out) if String::from_utf8_lossy(&out.stdout).trim() == "true" => {
            let track = Command::new("osascript")
                .arg("-e")
                .arg(r#"tell application "Spotify"
                    set t to name of current track
                    set a to artist of current track
                    set s to player state as string
                    return s & ": " & t & " by " & a
                end tell"#)
                .output();
            match track {
                Ok(out) => {
                    let info = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if info.is_empty() {
                        "Spotify: running but no track".to_string()
                    } else {
                        format!("Spotify: {}", info)
                    }
                }
                Err(_) => "Spotify: running".to_string(),
            }
        }
        _ => "Spotify: not running".to_string(),
    }
}
