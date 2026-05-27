use std::path::PathBuf;
use std::process::Command;

use super::module_loader::default_modules_dir;

fn find_mcp_script(module_id: &str) -> Option<PathBuf> {
    let path = default_modules_dir().join(module_id).join("mcp.py");
    if path.exists() { Some(path) } else { None }
}

pub fn has_mcp_module(module_id: &str) -> bool {
    find_mcp_script(module_id).is_some()
}

fn find_python() -> &'static str {
    static PYTHON: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PYTHON.get_or_init(|| {
        for candidate in &["python3", "/usr/bin/python3", "/opt/homebrew/bin/python3", "/usr/local/bin/python3"] {
            if Command::new(candidate).arg("--version").output().is_ok() {
                return candidate.to_string();
            }
        }
        "python3".to_string()
    })
}

pub fn call_mcp(module_id: &str, args: &[&str]) -> Result<serde_json::Value, String> {
    let script = find_mcp_script(module_id)
        .ok_or_else(|| format!("Module '{}' has no mcp.py", module_id))?;
    let python = find_python();
    let output = Command::new(python)
        .arg(&script)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run mcp.py (is python3 installed?): {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(if stderr.trim().is_empty() { "mcp.py failed".to_string() } else { stderr.trim().to_string() });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Invalid JSON from mcp.py: {} — got: {}", e, &stdout[..stdout.len().min(100)]))
}

// Generic Tauri commands

#[tauri::command]
pub fn is_mcp_module_present(module_id: String) -> bool {
    has_mcp_module(&module_id)
}

#[tauri::command]
pub async fn call_module_mcp(module_id: String, command: String, args: Vec<String>) -> Result<serde_json::Value, String> {
    let mut full_args: Vec<&str> = vec![&command];
    for a in &args {
        full_args.push(a);
    }
    call_mcp(&module_id, &full_args)
}

#[tauri::command]
pub async fn get_module_mcp_config(module_id: String) -> Result<serde_json::Value, String> {
    let config_path = module_data_dir(&module_id).join("config.json");
    let data = std::fs::read_to_string(&config_path).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_module_mcp_config(module_id: String, config: serde_json::Value) -> Result<(), String> {
    let config_path = module_data_dir(&module_id).join("config.json");
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, data).map_err(|e| e.to_string())
}

fn module_data_dir(module_id: &str) -> PathBuf {
    let dir = crate::paths::data_dir()
        .join(module_id.replace("amazon-internal", "amazon"));
    std::fs::create_dir_all(&dir).ok();
    dir
}
