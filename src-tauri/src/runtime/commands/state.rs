use super::RuntimeState;
use crate::runtime::activity_log::ActivityEntry;
use std::fs;
use tauri::State;

#[tauri::command]
pub fn get_activity_log(
    state: State<'_, RuntimeState>,
    limit: Option<usize>,
) -> Vec<ActivityEntry> {
    state.activity_logger.read_recent(limit.unwrap_or(50))
}

#[tauri::command]
pub fn get_screen_time(state: State<'_, RuntimeState>) -> std::collections::HashMap<String, u64> {
    let entries = state.activity_logger.read_recent(500);
    let mut times: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for entry in &entries {
        if entry.event == "app_switch" {
            *times.entry(entry.detail.clone()).or_insert(0) += 5;
        }
    }
    times
}

#[tauri::command]
pub async fn context_on_rage(state: State<'_, RuntimeState>) -> Result<(), String> {
    state.context_engine.write().await.on_rage();
    Ok(())
}

#[tauri::command]
pub async fn context_on_chat(state: State<'_, RuntimeState>) -> Result<(), String> {
    state.context_engine.write().await.on_chat();
    Ok(())
}

#[tauri::command]
pub async fn get_pet_context(state: State<'_, RuntimeState>) -> Result<String, String> {
    Ok(state.context_engine.read().await.get_context_string())
}

#[tauri::command]
pub fn log_status_change(
    state: State<'_, RuntimeState>,
    source: String,
    detail: String,
    delta: i32,
) {
    let entry = ActivityEntry {
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        source,
        event: "pet_status_changed".to_string(),
        detail,
        delta: Some(delta),
    };
    state.activity_logger.append(&entry);
}

#[tauri::command]
pub fn get_meow_nicknames() -> Vec<String> {
    crate::runtime::secret_meow::get_global_nicknames()
}

#[tauri::command]
pub fn get_system_stats() -> serde_json::Value {
    use std::process::Command;

    // Memory usage (this process)
    let pid = std::process::id();
    let mem = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|kb| format!("{:.1} MB", kb as f64 / 1024.0))
        .unwrap_or_else(|| "—".to_string());

    // CPU usage (this process, snapshot)
    let cpu = Command::new("ps")
        .args(["-o", "%cpu=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| format!("{}%", s.trim()))
        .unwrap_or_else(|| "—".to_string());

    // Uptime (process start time)
    let uptime = Command::new("ps")
        .args(["-o", "etime=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "—".to_string());

    serde_json::json!({
        "memory": mem,
        "cpu": cpu,
        "uptime": uptime,
    })
}

#[tauri::command]
pub fn reload_secret_meow() -> Result<(), String> {
    let state = crate::runtime::secret_meow::SecretMeowState::try_load();
    crate::runtime::secret_meow::reload_global_context(&state);
    Ok(())
}

#[tauri::command]
pub fn get_meow_public_info() -> serde_json::Value {
    let nicks = crate::runtime::secret_meow::get_global_nicknames();
    let personality = crate::runtime::secret_meow::get_global_personality_context();
    serde_json::json!({
        "nicknames": nicks,
        "personality": personality,
        "loaded": !nicks.is_empty(),
    })
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("Failed to open URL: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn clear_chat_history() -> Result<(), String> {
    let path = crate::paths::data_dir().join("chat_history.jsonl");
    if path.exists() {
        fs::write(&path, "").map_err(|e| format!("Failed to clear chat history: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn clear_activity_log(_state: State<'_, RuntimeState>) -> Result<(), String> {
    let data_dir = crate::paths::data_dir();
    let activity_path = data_dir.join("activity.jsonl");
    if activity_path.exists() {
        fs::write(&activity_path, "").map_err(|e| format!("Failed to clear activity log: {}", e))?;
    }
    let memory_dir = crate::paths::memory_dir();
    if memory_dir.exists() {
        for entry in fs::read_dir(&memory_dir).into_iter().flatten().flatten() {
            fs::remove_file(entry.path()).ok();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn export_logs() -> Result<String, String> {
    let data_dir = crate::paths::data_dir();
    let desktop = dirs::desktop_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let export_dir = desktop.join("ClaudeMeow_Export");
    fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;

    let files = ["activity.jsonl", "chat_history.jsonl"];
    for file in &files {
        let src = data_dir.join(file);
        if src.exists() {
            fs::copy(&src, export_dir.join(file)).ok();
        }
    }
    let config_src = crate::paths::config_dir().join("config.json");
    if config_src.exists() {
        fs::copy(&config_src, export_dir.join("config.json")).ok();
    }

    let dest = export_dir.to_string_lossy().to_string();
    std::process::Command::new("open").arg(&dest).spawn().ok();
    Ok(dest)
}

#[tauri::command]
pub fn open_data_folder() -> Result<(), String> {
    let data_dir = crate::paths::data_dir();
    fs::create_dir_all(&data_dir).ok();
    std::process::Command::new("open")
        .arg(data_dir.to_string_lossy().as_ref())
        .spawn()
        .map_err(|e| format!("Failed to open folder: {}", e))?;
    Ok(())
}
