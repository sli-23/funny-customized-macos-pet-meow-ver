use super::Platform;
use std::collections::HashMap;
use std::process::Command;

pub struct MacOsPlatform;

impl Platform for MacOsPlatform {
    fn get_active_window(&self) -> String {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events"
                set allProcs to every application process whose visible is true
                set frontApp to name of first application process whose frontmost is true
                set windowTitle to ""
                -- If ClaudeMeow is front, find the next visible app
                if frontApp is "claude-meow-pet" or frontApp is "ClaudeMeow" then
                    repeat with proc in allProcs
                        set procName to name of proc
                        if procName is not "claude-meow-pet" and procName is not "ClaudeMeow" then
                            set frontApp to procName
                            try
                                set windowTitle to name of front window of proc
                            end try
                            exit repeat
                        end if
                    end repeat
                else
                    try
                        tell process frontApp
                            set windowTitle to name of front window
                        end tell
                    end try
                end if
                return frontApp & " - " & windowTitle
            end tell"#)
            .output();

        match output {
            Ok(out) => {
                let result = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if result.is_empty() || result == " - " {
                    "Unknown".to_string()
                } else {
                    result
                }
            }
            Err(_) => "Unknown".to_string(),
        }
    }

    fn is_typing(&self) -> bool {
        let out = Command::new("ioreg")
            .args(["-r", "-k", "HIDIdleTime", "-n", "IOHIDKeyboard"])
            .output()
            .ok();
        if let Some(output) = out {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("HIDIdleTime") {
                    if let Some(val) = line.split_whitespace().last() {
                        if let Ok(ns) = val.parse::<u64>() {
                            return ns / 1_000_000_000 < 2;
                        }
                    }
                }
            }
        }
        // Fallback: use general idle time if keyboard-specific unavailable
        match self.get_idle_seconds() {
            Some(secs) => secs < 2,
            None => false,
        }
    }

    fn get_battery_info(&self) -> Option<String> {
        let out = Command::new("pmset").args(["-g", "batt"]).output().ok()?;
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
                    return Some(format!("Battery: {}{}", pct, charging));
                }
            }
        }
        None
    }

    fn get_idle_seconds(&self) -> Option<u64> {
        let out = Command::new("ioreg").args(["-c", "IOHIDSystem"]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("HIDIdleTime") {
                if let Some(val) = line.split_whitespace().last() {
                    if let Ok(ns) = val.parse::<u64>() {
                        return Some(ns / 1_000_000_000);
                    }
                }
            }
        }
        None
    }

    fn get_browser_url(&self, app_name: &str) -> Option<String> {
        let lower = app_name.to_lowercase();
        let script = if lower.contains("chrome") {
            r#"tell application "Google Chrome" to get URL of active tab of front window"#
        } else if lower.contains("arc") {
            r#"tell application "Arc" to get URL of active tab of front window"#
        } else if lower.contains("safari") {
            r#"tell application "Safari" to get URL of front document"#
        } else if lower.contains("edge") {
            r#"tell application "Microsoft Edge" to get URL of active tab of front window"#
        } else if lower.contains("firefox") {
            return None;
        } else if lower.contains("brave") {
            r#"tell application "Brave Browser" to get URL of active tab of front window"#
        } else {
            return None;
        };
        self.run_applescript(script)
    }

    fn is_process_running(&self, name: &str) -> bool {
        Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn run_applescript(&self, script: &str) -> Option<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text.is_empty() { None } else { Some(text) }
        } else {
            None
        }
    }

    fn check_permissions_status(&self) -> HashMap<String, bool> {
        let mut result = HashMap::new();

        let accessibility = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events" to get name of first application process whose frontmost is true"#)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        result.insert("accessibility".to_string(), accessibility);

        let window_title = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events"
                set frontApp to name of first application process whose frontmost is true
                tell process frontApp
                    set wTitle to name of front window
                end tell
                return wTitle
            end tell"#)
            .output()
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);
        result.insert("window_titles".to_string(), window_title);

        let chrome_running = self.is_process_running("Google Chrome");
        if chrome_running {
            let chrome_url = self.run_applescript(
                r#"tell application "Google Chrome" to get URL of active tab of front window"#
            ).is_some();
            result.insert("browser_url".to_string(), chrome_url);
        } else {
            result.insert("browser_url".to_string(), false);
        }

        result
    }

    fn open_system_settings(&self, url: &str) {
        Command::new("open").arg(url).spawn().ok();
    }
}
