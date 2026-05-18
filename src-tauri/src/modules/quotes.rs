use std::process::Command;

pub fn get_context() -> String {
    // Try Chinese quote first (hitokoto), then English (zenquotes)
    if let Some(q) = get_hitokoto() {
        return q;
    }
    if let Some(q) = get_zenquote() {
        return q;
    }
    String::new()
}

fn get_hitokoto() -> Option<String> {
    let output = Command::new("curl")
        .args(["-s", "--max-time", "3", "https://v1.hitokoto.cn/?c=k&encode=text"])
        .output()
        .ok()?;
    let quote = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if quote.is_empty() || quote.len() > 100 {
        None
    } else {
        Some(quote)
    }
}

fn get_zenquote() -> Option<String> {
    let output = Command::new("curl")
        .args(["-s", "--max-time", "3", "https://zenquotes.io/api/random"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&text) {
        let q = arr[0]["q"].as_str()?;
        if q.len() > 80 {
            return None;
        }
        return Some(q.to_string());
    }
    None
}
