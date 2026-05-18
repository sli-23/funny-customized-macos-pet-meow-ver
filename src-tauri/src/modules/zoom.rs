use std::process::Command;

pub fn get_context() -> String {
    let running = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to (name of processes) contains "zoom.us""#)
        .output();

    match running {
        Ok(out) if String::from_utf8_lossy(&out.stdout).trim() == "true" => {
            let meeting = Command::new("osascript")
                .arg("-e")
                .arg(r#"tell application "System Events"
                    tell process "zoom.us"
                        set winNames to name of every window
                        set output to ""
                        repeat with w in winNames
                            set output to output & w & ", "
                        end repeat
                        return output
                    end tell
                end tell"#)
                .output();
            match meeting {
                Ok(out) => {
                    let info = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if info.contains("Meeting") || info.contains("Zoom Meeting") || info.contains("meeting") {
                        "Zoom: in a meeting".to_string()
                    } else if info.is_empty() {
                        "Zoom: running (no meeting)".to_string()
                    } else {
                        format!("Zoom: running ({})", info.trim_end_matches(", "))
                    }
                }
                Err(_) => "Zoom: running".to_string(),
            }
        }
        _ => "Zoom: not running".to_string(),
    }
}
