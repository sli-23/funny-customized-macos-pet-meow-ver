use crate::platform::{self, Platform};

pub fn get_context() -> String {
    let plat = platform::native();

    if !plat.is_process_running("zoom.us") {
        return "Zoom: not running".to_string();
    }

    let script = r#"tell application "System Events"
        tell process "zoom.us"
            set winNames to name of every window
            set output to ""
            repeat with w in winNames
                set output to output & w & ", "
            end repeat
            return output
        end tell
    end tell"#;

    match plat.run_applescript(script) {
        Some(info) => {
            if info.contains("Meeting") || info.contains("Zoom Meeting") || info.contains("meeting") {
                "Zoom: in a meeting".to_string()
            } else {
                format!("Zoom: running ({})", info.trim_end_matches(", "))
            }
        }
        None => "Zoom: running (no meeting)".to_string(),
    }
}
