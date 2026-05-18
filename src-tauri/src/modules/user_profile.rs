use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── Secret .meow profile (from a friend) ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeowProfile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub nicknames: Vec<String>,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub favorites: serde_json::Value,
    #[serde(default)]
    pub secret_messages: Vec<String>,
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-meow-pet")
}

fn profile_path() -> PathBuf {
    config_dir().join("user_profile.md")
}

fn profiles_dir() -> PathBuf {
    config_dir().join("profiles")
}

// ── Context for AI (combines both sources) ──

pub fn get_context() -> String {
    let mut parts = Vec::new();

    // 1. User's own input (from settings "About You" — what they wrote themselves)
    let path = profile_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if !content.trim().is_empty() {
                parts.push(format!("The user describes themselves as: {}", content.trim()));
            }
        }
    }

    // 2. Secret .meow profile (written by a friend who knows them — more personal and intimate)
    if let Some(meow) = load_meow_profile() {
        if !meow.personality.is_empty() {
            parts.push(format!("A close friend says about this user: {}", meow.personality));
        }
        if let Some(obj) = meow.favorites.as_object() {
            for (k, v) in obj {
                parts.push(format!("Their friend says they love {}: {}", k, v));
            }
        }
    }

    parts.join("\n")
}

pub fn load_meow_profile() -> Option<MeowProfile> {
    let dir = profiles_dir();
    if !dir.exists() {
        return None;
    }
    for entry in fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("meow") {
            if let Ok(data) = fs::read_to_string(&path) {
                // Try parsing as plain JSON first
                if let Ok(profile) = serde_json::from_str::<MeowProfile>(&data) {
                    return Some(profile);
                }
                // Try Base64 decoding
                if let Ok(decoded_bytes) = base64_decode(data.trim()) {
                    if let Ok(json_str) = String::from_utf8(decoded_bytes) {
                        if let Ok(profile) = serde_json::from_str::<MeowProfile>(&json_str) {
                            return Some(profile);
                        }
                    }
                }
            }
        }
    }
    None
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    use std::collections::HashMap;
    let table: HashMap<u8, u8> = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter().enumerate().map(|(i, &c)| (c, i as u8)).collect();
    let input = input.as_bytes();
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in input {
        if b == b'=' || b == b'\n' || b == b'\r' { continue; }
        let val = *table.get(&b).ok_or(())?;
        buf = (buf << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

// ── Tauri commands ──

#[tauri::command]
pub fn get_user_profile() -> String {
    fs::read_to_string(profile_path()).unwrap_or_default()
}

#[tauri::command]
pub fn set_user_profile(content: String) -> Result<(), String> {
    let path = profile_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_meow_profile() -> Option<MeowProfile> {
    load_meow_profile()
}

#[tauri::command]
pub fn import_meow_profile(content: String) -> Result<String, String> {
    let profile: MeowProfile =
        serde_json::from_str(&content).map_err(|e| format!("Invalid .meow file: {}", e))?;
    let dir = profiles_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let filename = format!("{}.meow", profile.name.to_lowercase().replace(' ', "_"));
    fs::write(dir.join(&filename), &content).map_err(|e| e.to_string())?;
    Ok(profile.name)
}
