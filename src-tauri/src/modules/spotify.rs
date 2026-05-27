use crate::platform::{self, Platform};

pub fn get_context() -> String {
    let plat = platform::native();

    if !plat.is_process_running("Spotify") {
        return "Spotify: not running".to_string();
    }

    let script = r#"tell application "Spotify"
        set t to name of current track
        set a to artist of current track
        set s to player state as string
        return s & ": " & t & " by " & a
    end tell"#;

    match plat.run_applescript(script) {
        Some(info) => format!("Spotify: {}", info),
        None => "Spotify: running but no track".to_string(),
    }
}
