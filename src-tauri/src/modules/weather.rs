use std::process::Command;

pub fn get_context() -> String {
    let output = Command::new("curl")
        .args(["-s", "--max-time", "3", "wttr.in/Seattle?format=%C+%t+%h+%w"])
        .output();

    match output {
        Ok(out) => {
            let weather = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if weather.is_empty() || weather.contains("Unknown") || weather.contains("<!DOCTYPE") {
                "Weather: unavailable".to_string()
            } else {
                format!("Weather in Seattle: {}", weather)
            }
        }
        Err(_) => "Weather: unavailable".to_string(),
    }
}
