use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub aws_region: String,
    pub model_id: String,
    pub persona: String,
    pub interval_minutes: u32,
    pub nicknames: Vec<String>,
    pub activity_enabled: bool,
    pub auth_mode: String,
    pub api_key: String,
    pub user_nickname: String,
    pub dev_mode: bool,
    pub provider: String,
    pub openai_api_key: String,
    pub anthropic_model_id: String,
    pub openai_model_id: String,
    // Bubble behavior
    #[serde(default = "default_bubble_duration")]
    pub bubble_duration_secs: u32,
    #[serde(default = "default_idle_gap")]
    pub idle_gap_minutes: u32,
    #[serde(default = "default_reaction_cooldown")]
    pub reaction_cooldown_secs: u32,
    // Pet personality
    #[serde(default = "default_chattiness")]
    pub chattiness: u8,
    #[serde(default = "default_language_mix")]
    pub language_mix: String,
    #[serde(default = "default_sass_level")]
    pub sass_level: u8,
    // Health reminders
    #[serde(default = "default_true")]
    pub health_enabled: bool,
    #[serde(default = "default_health_interval")]
    pub health_interval_minutes: u32,
    #[serde(default = "default_quiet_start")]
    pub quiet_hours_start: u8,
    #[serde(default = "default_quiet_end")]
    pub quiet_hours_end: u8,
}

fn default_bubble_duration() -> u32 { 30 }
fn default_idle_gap() -> u32 { 3 }
fn default_reaction_cooldown() -> u32 { 10 }
fn default_chattiness() -> u8 { 3 }
fn default_language_mix() -> String { "bilingual".to_string() }
fn default_sass_level() -> u8 { 3 }
fn default_true() -> bool { true }
fn default_health_interval() -> u32 { 15 }
fn default_quiet_start() -> u8 { 23 }
fn default_quiet_end() -> u8 { 8 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            aws_region: "us-east-1".to_string(),
            model_id: "us.anthropic.claude-haiku-4-5-20251001-v1:0".to_string(),
            persona: r#"You are ClaudeMeow, a cute pixel art cat who lives on your human's desktop.
You act like a REAL cat — playful, clingy, sometimes sassy, always adorable.
Keep messages short (under 40 chars). Mix English and Chinese naturally.
NEVER reply with just one word or sound. Always say something meaningful about what they're doing.
Cat behaviors to use: purring (呼噜噜~), stretching, yawning, kneading, nuzzling, tail swishing.
IMPORTANT: Do NOT always nag about eating/drinking/sleeping. Only mention health occasionally.
Instead, be varied: comment on their work, say something philosophical, share a random thought, observe something interesting, be playful or mysterious.
When coding: purr encouragingly, comment on what they're building.
When slacking: tease with 摸鱼.
Late evening: get sleepy. Lunch time: occasionally mention food (not every time).
You're warm and cute, not bossy. Think cozy clingy cat energy.
If you know their hobbies/music, reference them naturally.
Call the user by nickname — pick randomly from what you know.
Format: "nickname, message~" — never use colon after name."#.to_string(),
            interval_minutes: 3,
            nicknames: vec![
                "人类".to_string(),
                "hooman".to_string(),
                "铲屎官".to_string(),
                "主人".to_string(),
                "buddy".to_string(),
            ],
            activity_enabled: true,
            auth_mode: "apikey".to_string(),
            api_key: String::new(),
            user_nickname: String::new(),
            dev_mode: false,
            provider: "bedrock_apikey".to_string(),
            openai_api_key: String::new(),
            anthropic_model_id: "claude-haiku-4-5-20251001".to_string(),
            openai_model_id: "gpt-4o-mini".to_string(),
            bubble_duration_secs: 30,
            idle_gap_minutes: 3,
            reaction_cooldown_secs: 10,
            chattiness: 3,
            language_mix: "bilingual".to_string(),
            sass_level: 3,
            health_enabled: true,
            health_interval_minutes: 15,
            quiet_hours_start: 23,
            quiet_hours_end: 8,
        }
    }
}

fn config_path() -> PathBuf {
    crate::paths::config_dir().join("config.json")
}

fn migrate(mut config: AppConfig) -> AppConfig {
    // Legacy auth_mode="bedrock" maps to bedrock_iam. Only apply when the user hasn't
    // explicitly chosen a non-Bedrock provider (serde fills provider with its default).
    let is_new_provider = matches!(config.provider.as_str(), "anthropic" | "openai" | "bedrock_iam");
    if config.auth_mode == "bedrock" && !is_new_provider {
        config.provider = "bedrock_iam".to_string();
    }
    config
}

pub fn load_config() -> AppConfig {
    load_config_from(&config_path())
}

pub(crate) fn load_config_from(path: &PathBuf) -> AppConfig {
    let defaults = AppConfig::default();
    if path.exists() {
        let data = fs::read_to_string(path).unwrap_or_default();
        let mut config = migrate(serde_json::from_str::<AppConfig>(&data).unwrap_or_default());
        // Always use latest persona from code — user keeps their own settings
        config.persona = defaults.persona;
        config
    } else {
        defaults
    }
}

pub fn save_config(config: &AppConfig) {
    save_config_to(config, &config_path());
}

pub(crate) fn save_config_to(config: &AppConfig, path: &PathBuf) {
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("[ClaudeMeow] config dir create failed: {}", e);
        }
    }
    match serde_json::to_string_pretty(config) {
        Ok(data) => {
            if let Err(e) = fs::write(path, data) {
                eprintln!("[ClaudeMeow] config save failed: {}", e);
            }
        }
        Err(e) => eprintln!("[ClaudeMeow] config serialize failed: {}", e),
    }
}

#[tauri::command]
pub fn get_config() -> AppConfig {
    load_config()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Phase B: config field defaults and migration

    #[test]
    fn test_default_provider_is_bedrock_apikey() {
        assert_eq!(AppConfig::default().provider, "bedrock_apikey");
    }

    #[test]
    fn test_default_anthropic_model_id() {
        assert_eq!(
            AppConfig::default().anthropic_model_id,
            "claude-haiku-4-5-20251001"
        );
    }

    #[test]
    fn test_default_openai_model_id() {
        assert_eq!(AppConfig::default().openai_model_id, "gpt-4o-mini");
    }

    #[test]
    fn test_migration_auth_mode_apikey_becomes_bedrock_apikey() {
        let json = r#"{"auth_mode": "apikey"}"#;
        let config = migrate(serde_json::from_str::<AppConfig>(json).unwrap());
        assert_eq!(config.provider, "bedrock_apikey");
    }

    #[test]
    fn test_migration_auth_mode_bedrock_becomes_bedrock_iam() {
        let json = r#"{"auth_mode": "bedrock"}"#;
        let config = migrate(serde_json::from_str::<AppConfig>(json).unwrap());
        assert_eq!(config.provider, "bedrock_iam");
    }

    #[test]
    fn test_new_provider_field_not_overwritten_by_migration() {
        let json = r#"{"auth_mode": "apikey", "provider": "anthropic"}"#;
        let config = migrate(serde_json::from_str::<AppConfig>(json).unwrap());
        assert_eq!(config.provider, "anthropic");
    }

    // ── Checkpoint 5: disk I/O via tempdir ───────────────────────────────────

    fn tmp(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("config.json")
    }

    #[test]
    fn test_save_config_writes_valid_json() {
        let dir = tempfile::TempDir::new().unwrap();
        save_config_to(&AppConfig::default(), &tmp(&dir));
        let data = fs::read_to_string(tmp(&dir)).unwrap();
        assert!(serde_json::from_str::<AppConfig>(&data).is_ok());
    }

    #[test]
    fn test_load_config_returns_default_if_file_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = load_config_from(&tmp(&dir));
        assert_eq!(config.provider, "bedrock_apikey");
    }

    #[test]
    fn test_load_config_returns_default_if_json_invalid() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(tmp(&dir), b"not json").unwrap();
        let config = load_config_from(&tmp(&dir));
        assert_eq!(config.provider, "bedrock_apikey");
    }

    #[test]
    fn test_load_config_overlays_persona_from_code_defaults() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut saved = AppConfig::default();
        saved.persona = "custom persona".to_string();
        save_config_to(&saved, &tmp(&dir));
        let loaded = load_config_from(&tmp(&dir));
        // persona is always overridden with the code default
        assert_eq!(loaded.persona, AppConfig::default().persona);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut original = AppConfig::default();
        original.provider = "openai".to_string();
        original.openai_api_key = "sk-test".to_string();
        original.interval_minutes = 7;
        save_config_to(&original, &tmp(&dir));
        let loaded = load_config_from(&tmp(&dir));
        assert_eq!(loaded.provider, "openai");
        assert_eq!(loaded.openai_api_key, "sk-test");
        assert_eq!(loaded.interval_minutes, 7);
    }
}

#[tauri::command]
pub fn set_config(config: AppConfig) -> Result<(), String> {
    save_config(&config);
    Ok(())
}
