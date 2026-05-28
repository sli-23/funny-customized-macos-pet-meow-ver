use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::Mutex;
use tauri::Emitter;

use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

use crate::modules::user_profile::MeowProfile;

const MEOW2_HEADER: &[u8] = b"MEOW2\n";
const APP_SECRET: &[u8] = b"ClaudeMeowPet2024SecretSalt!!xK9";
const PBKDF2_ITERATIONS: u32 = 10_000;
const SECRET_MESSAGE_INTERVAL_SECS: u64 = 1800; // 30 min

pub struct SecretMeowState {
    pub loaded: bool,
    pub nicknames: Vec<String>,
    personality: String,
    favorites: serde_json::Value,
    secret_messages: Vec<String>,
    secret_idx: usize,
    last_secret_fired: u64,
}

impl SecretMeowState {
    pub fn new() -> Self {
        Self {
            loaded: false,
            nicknames: Vec::new(),
            personality: String::new(),
            favorites: serde_json::Value::Null,
            secret_messages: Vec::new(),
            secret_idx: 0,
            last_secret_fired: 0,
        }
    }

    pub fn try_load() -> Self {
        let dir = crate::paths::profiles_dir();
        if !dir.exists() {
            return Self::new();
        }

        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Self::new(),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("meow") {
                continue;
            }

            if let Some(profile) = load_from_path(&path) {
                return Self {
                    loaded: true,
                    nicknames: profile.nicknames,
                    personality: profile.personality,
                    favorites: profile.favorites,
                    secret_messages: profile.secret_messages,
                    secret_idx: 0,
                    last_secret_fired: crate::util::now_secs(),
                };
            }
        }

        Self::new()
    }

    pub fn get_personality_context(&self) -> String {
        if !self.loaded {
            return String::new();
        }

        let mut parts = Vec::new();

        if !self.personality.is_empty() {
            parts.push(format!("A close friend says about this user: {}", self.personality));
        }

        if let Some(obj) = self.favorites.as_object() {
            for (k, v) in obj {
                parts.push(format!("Their friend says they love {}: {}", k, v));
            }
        }

        parts.join("\n")
    }

    pub fn tick(&mut self, app: &tauri::AppHandle, now_secs: u64) {
        if !self.loaded || self.secret_messages.is_empty() {
            return;
        }

        if now_secs - self.last_secret_fired < SECRET_MESSAGE_INTERVAL_SECS {
            return;
        }

        self.last_secret_fired = now_secs;
        let msg = &self.secret_messages[self.secret_idx % self.secret_messages.len()];
        self.secret_idx += 1;

        let _ = app.emit("module-reaction", serde_json::json!({
            "module_id": "secret",
            "message": msg,
            "priority": 6,
        }));
    }
}

// ── Global accessor for personality context (used by user_profile.rs) ──

static GLOBAL_CONTEXT: OnceLock<Mutex<(String, Vec<String>)>> = OnceLock::new();

pub fn init_global_context(state: &SecretMeowState) {
    let ctx = state.get_personality_context();
    let nicks = state.nicknames.clone();
    let _ = GLOBAL_CONTEXT.set(Mutex::new((ctx, nicks)));
}

pub fn reload_global_context(state: &SecretMeowState) {
    let ctx = state.get_personality_context();
    let nicks = state.nicknames.clone();
    if let Some(global) = GLOBAL_CONTEXT.get() {
        if let Ok(mut g) = global.lock() {
            *g = (ctx, nicks);
        }
    } else {
        let _ = GLOBAL_CONTEXT.set(Mutex::new((ctx, nicks)));
    }
}

pub fn get_global_personality_context() -> String {
    GLOBAL_CONTEXT.get()
        .and_then(|m| m.lock().ok())
        .map(|g| g.0.clone())
        .unwrap_or_default()
}

pub fn get_global_nicknames() -> Vec<String> {
    GLOBAL_CONTEXT.get()
        .and_then(|m| m.lock().ok())
        .map(|g| g.1.clone())
        .unwrap_or_default()
}

// ── File loading ──

fn load_from_path(path: &PathBuf) -> Option<MeowProfile> {
    let data = fs::read(path).ok()?;

    // Check for MEOW2 encrypted format
    if data.starts_with(MEOW2_HEADER) {
        return decrypt_meow2(&data[MEOW2_HEADER.len()..]);
    }

    // Fallback: plaintext JSON
    if let Ok(text) = String::from_utf8(data.clone()) {
        if let Ok(profile) = serde_json::from_str::<MeowProfile>(&text) {
            return Some(profile);
        }
        // Try base64
        if let Ok(decoded) = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            text.trim().as_bytes(),
        ) {
            if let Ok(json_str) = String::from_utf8(decoded) {
                if let Ok(profile) = serde_json::from_str::<MeowProfile>(&json_str) {
                    return Some(profile);
                }
            }
        }
    }

    None
}

fn decrypt_meow2(encrypted_data: &[u8]) -> Option<MeowProfile> {
    if encrypted_data.len() < 12 {
        return None;
    }

    // First 12 bytes = nonce, rest = ciphertext + auth tag
    let nonce_bytes = &encrypted_data[..12];
    let ciphertext = &encrypted_data[12..];

    // Derive AES key from hardcoded secret
    let key = derive_key(APP_SECRET);

    // Decrypt
    let cipher = Aes256Gcm::new_from_slice(&key).ok()?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;

    // Parse JSON
    let json_str = String::from_utf8(plaintext).ok()?;
    serde_json::from_str::<MeowProfile>(&json_str).ok()
}

fn derive_key(secret: &[u8]) -> [u8; 32] {
    let salt = b"ClaudeMeowPBKDF2Salt2024";
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(secret, salt, PBKDF2_ITERATIONS, &mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_consistent() {
        let key1 = derive_key(APP_SECRET);
        let key2 = derive_key(APP_SECRET);
        assert_eq!(key1, key2);
        // Print for comparison with Python
        let hex: String = key1.iter().map(|b| format!("{:02x}", b)).collect();
        eprintln!("Rust derived key: {}", hex);
    }


    #[test]
    fn test_decrypt_invalid_data_returns_none() {
        let result = decrypt_meow2(b"garbage data that is not encrypted");
        assert!(result.is_none());
    }

    #[test]
    fn test_load_plaintext_json() {
        let dir = tempfile::tempdir().unwrap();
        let meow_path = dir.path().join("test.meow");
        let json = r#"{"name":"Test","nicknames":["TestNick"],"personality":"test personality","favorites":{},"secret_messages":["msg1"]}"#;
        fs::write(&meow_path, json).unwrap();

        let profile = load_from_path(&meow_path).unwrap();
        assert_eq!(profile.name, "Test");
        assert_eq!(profile.nicknames, vec!["TestNick"]);
        assert_eq!(profile.secret_messages, vec!["msg1"]);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let json = r#"{"name":"Kate","nicknames":["KatMeow","喵喵"],"personality":"black cat energy","favorites":{"music":["indie"]},"secret_messages":["secret1","secret2"]}"#;

        // Encrypt (same logic as meow_encrypt.py)
        let key = derive_key(APP_SECRET);
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce_bytes: [u8; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]; // deterministic for test
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, json.as_bytes()).unwrap();

        // Build encrypted blob (nonce + ciphertext)
        let mut encrypted = Vec::new();
        encrypted.extend_from_slice(&nonce_bytes);
        encrypted.extend_from_slice(&ciphertext);

        // Decrypt
        let profile = decrypt_meow2(&encrypted).unwrap();
        assert_eq!(profile.name, "Kate");
        assert_eq!(profile.nicknames, vec!["KatMeow", "喵喵"]);
        assert_eq!(profile.personality, "black cat energy");
        assert_eq!(profile.secret_messages, vec!["secret1", "secret2"]);
    }
}
