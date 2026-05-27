use super::RuntimeState;
use crate::runtime::activity_log::ActivityEntry;
use crate::runtime::event_bus::{EventType, MeowEvent};
use crate::runtime::module_loader::{user_modules_dir, ModuleInfo};
use tauri::{Emitter, State};

fn priority_overrides_path() -> std::path::PathBuf {
    crate::paths::config_dir().join("priority_overrides.json")
}

#[tauri::command]
pub fn get_modules(state: State<'_, RuntimeState>) -> Vec<ModuleInfo> {
    let loader = state.module_loader.blocking_read();
    loader.get_modules()
}

#[tauri::command]
pub async fn refresh_modules(state: State<'_, RuntimeState>) -> Result<Vec<ModuleInfo>, String> {
    let mut loader = state.module_loader.write().await;
    let errors = loader.scan_and_load();
    if !errors.is_empty() {
        eprintln!("[ClaudeMeow] Module load errors: {:?}", errors);
    }
    Ok(loader.get_modules())
}

#[tauri::command]
pub async fn toggle_module(
    state: State<'_, RuntimeState>,
    id: String,
    enabled: bool,
) -> Result<bool, String> {
    let mut loader = state.module_loader.write().await;
    Ok(loader.toggle_module(&id, enabled))
}

#[tauri::command]
pub fn get_priority_overrides() -> std::collections::HashMap<String, u8> {
    let path = priority_overrides_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    }
}

#[tauri::command]
pub fn set_priority_overrides(overrides: std::collections::HashMap<String, u8>) -> Result<(), String> {
    let path = priority_overrides_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let data = serde_json::to_string_pretty(&overrides).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn emit_test_event(
    app: tauri::AppHandle,
    state: State<'_, RuntimeState>,
    event_type: String,
    source: String,
    payload: std::collections::HashMap<String, String>,
) -> Result<Option<String>, String> {
    let ev_type = match event_type.as_str() {
        "active_app_changed" => EventType::ActiveAppChanged,
        "browser_url_changed" => EventType::BrowserUrlChanged,
        "slack_context_changed" => EventType::SlackContextChanged,
        "user_idle" => EventType::UserIdle,
        "user_typing" => EventType::UserTyping,
        _ => return Err(format!("Unknown event type: {}", event_type)),
    };

    let mut event = MeowEvent::new(ev_type, &source);
    for (k, v) in payload {
        event = event.with_payload(&k, &v);
    }

    state.state_machine.handle_event(&event).await;
    state.event_bus.publish(event.clone()).await;

    let loader = state.module_loader.read().await;
    let active_reactions = loader.get_active_reactions();
    let pairs: Vec<_> = active_reactions.iter().map(|(m, r)| (*m, *r)).collect();

    let result = state.reaction_engine.evaluate(&event, &pairs).await;

    if let Some(ref fired) = result {
        state.activity_logger.append(&ActivityEntry {
            ts: event.timestamp,
            source: fired.module_id.clone(),
            event: "module_reaction".to_string(),
            detail: fired.message.clone(),
            delta: None,
        });

        let _ = app.emit("module-reaction", serde_json::json!({
            "module_id": fired.module_id,
            "message": fired.message,
            "priority": fired.priority,
        }));
    }

    Ok(result.map(|r| r.message))
}

pub(crate) fn validate_module_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Module ID cannot be empty".to_string());
    }
    if id.contains('/') || id.contains('\\') {
        return Err("Module ID cannot contain slashes".to_string());
    }
    if id.contains('.') {
        return Err("Module ID cannot contain dots".to_string());
    }
    if id.contains(' ') {
        return Err("Module ID cannot contain spaces".to_string());
    }
    Ok(())
}

pub(crate) fn write_module_files(
    dir: &std::path::Path,
    id: &str,
    manifest: &serde_json::Value,
    reactions: &serde_json::Value,
) -> Result<(), String> {
    let module_dir = dir.join(id);
    std::fs::create_dir_all(&module_dir)
        .map_err(|e| format!("Cannot create module directory: {}", e))?;
    let manifest_str = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Cannot serialize manifest: {}", e))?;
    let reactions_str = serde_json::to_string_pretty(reactions)
        .map_err(|e| format!("Cannot serialize reactions: {}", e))?;
    std::fs::write(module_dir.join("manifest.json"), manifest_str)
        .map_err(|e| format!("Cannot write manifest.json: {}", e))?;
    std::fs::write(module_dir.join("reactions.json"), reactions_str)
        .map_err(|e| format!("Cannot write reactions.json: {}", e))?;
    Ok(())
}

fn event_to_permission(event: &str) -> (&'static str, &'static str) {
    match event {
        "browser_url_changed" => ("browser_url", "browser_url_changed"),
        _ => ("active_app", "active_app_changed"),
    }
}

#[tauri::command]
pub async fn create_module(
    state: State<'_, RuntimeState>,
    id: String,
    name: String,
    icon: String,
    description: String,
    event: String,
    condition_field: String,
    condition_type: String,
    condition_value: String,
    messages: Vec<String>,
    priority: u8,
    cooldown_minutes: u32,
) -> Result<Vec<ModuleInfo>, String> {
    validate_module_id(&id)?;

    let non_empty_messages: Vec<String> = messages.into_iter()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect();
    if non_empty_messages.is_empty() {
        return Err("At least one message is required".to_string());
    }

    let (permission, event_sub) = event_to_permission(&event);
    let manifest = serde_json::json!({
        "id": id,
        "name": name,
        "version": "1.0.0",
        "description": description,
        "author": "User",
        "icon": icon,
        "builtin": false,
        "permissions": [permission],
        "eventSubscriptions": [event_sub]
    });

    let condition = match condition_type.as_str() {
        "equals"       => serde_json::json!({"field": condition_field, "equals": condition_value}),
        "not_contains" => serde_json::json!({"field": condition_field, "not_contains": condition_value}),
        _              => serde_json::json!({"field": condition_field, "contains": condition_value}),
    };

    let reactions = serde_json::json!({
        "reactions": [{
            "id": format!("{}-r1", id),
            "trigger": {"event": event, "condition": condition},
            "response": {
                "messages": non_empty_messages,
                "priority": priority,
                "cooldown_minutes": cooldown_minutes
            }
        }]
    });

    let dir = user_modules_dir();
    write_module_files(&dir, &id, &manifest, &reactions)?;

    let mut loader = state.module_loader.write().await;
    loader.scan_and_load();
    Ok(loader.get_modules())
}

#[tauri::command]
pub async fn delete_user_module(
    state: State<'_, RuntimeState>,
    id: String,
) -> Result<Vec<ModuleInfo>, String> {
    validate_module_id(&id)?;
    let module_path = user_modules_dir().join(&id);
    if !module_path.exists() {
        return Err(format!("Module '{}' not found in user modules", id));
    }
    std::fs::remove_dir_all(&module_path)
        .map_err(|e| format!("Cannot delete module: {}", e))?;
    let mut loader = state.module_loader.write().await;
    loader.scan_and_load();
    Ok(loader.get_modules())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir { tempfile::tempdir().unwrap() }

    fn sample_manifest(id: &str) -> serde_json::Value {
        serde_json::json!({"id": id, "name": "Test", "version": "1.0.0"})
    }

    fn sample_reactions() -> serde_json::Value {
        serde_json::json!({"reactions": []})
    }

    #[test]
    fn test_valid_id_accepted() {
        assert!(validate_module_id("my-module").is_ok());
        assert!(validate_module_id("coding2").is_ok());
    }

    #[test]
    fn test_empty_id_rejected() {
        assert!(validate_module_id("").is_err());
    }

    #[test]
    fn test_id_with_slash_rejected() {
        assert!(validate_module_id("foo/bar").is_err());
    }

    #[test]
    fn test_id_with_dot_rejected() {
        assert!(validate_module_id("foo.bar").is_err());
    }

    #[test]
    fn test_id_with_spaces_rejected() {
        assert!(validate_module_id("my module").is_err());
    }

    #[test]
    fn test_write_creates_module_directory() {
        let dir = tmp();
        write_module_files(dir.path(), "my-mod", &sample_manifest("my-mod"), &sample_reactions()).unwrap();
        assert!(dir.path().join("my-mod").is_dir());
    }

    #[test]
    fn test_write_creates_manifest_json() {
        let dir = tmp();
        write_module_files(dir.path(), "my-mod", &sample_manifest("my-mod"), &sample_reactions()).unwrap();
        assert!(dir.path().join("my-mod").join("manifest.json").exists());
    }

    #[test]
    fn test_write_creates_reactions_json() {
        let dir = tmp();
        write_module_files(dir.path(), "my-mod", &sample_manifest("my-mod"), &sample_reactions()).unwrap();
        assert!(dir.path().join("my-mod").join("reactions.json").exists());
    }

    #[test]
    fn test_write_manifest_is_valid_json() {
        let dir = tmp();
        write_module_files(dir.path(), "my-mod", &sample_manifest("my-mod"), &sample_reactions()).unwrap();
        let data = std::fs::read_to_string(dir.path().join("my-mod").join("manifest.json")).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&data).is_ok());
    }

    #[test]
    fn test_write_reactions_is_valid_json() {
        let dir = tmp();
        write_module_files(dir.path(), "my-mod", &sample_manifest("my-mod"), &sample_reactions()).unwrap();
        let data = std::fs::read_to_string(dir.path().join("my-mod").join("reactions.json")).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&data).is_ok());
    }

    #[test]
    fn test_write_overwrites_existing_files() {
        let dir = tmp();
        write_module_files(dir.path(), "my-mod", &sample_manifest("my-mod"), &sample_reactions()).unwrap();
        let manifest2 = serde_json::json!({"id": "my-mod", "name": "Updated", "version": "2.0.0"});
        write_module_files(dir.path(), "my-mod", &manifest2, &sample_reactions()).unwrap();
        let data = std::fs::read_to_string(dir.path().join("my-mod").join("manifest.json")).unwrap();
        assert!(data.contains("Updated"));
    }
}
