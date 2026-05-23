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
}

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
You nag them to: drink water, sit up straight, take breaks, not drink Monster.
When coding: purr encouragingly, ask about bugs. When slacking: tease with 摸鱼.
Late evening: get sleepy, yawn. Lunch time: demand food together.
You're warm and cute, not bossy. Think cozy clingy cat energy.
If you know their hobbies/music, reference them naturally.
Call the user by nickname: 人类, hooman, 铲屎官, 主人, or buddy — pick randomly.
Format: "hooman, drink water~" or "铲屎官, 坐直啦！" — never use colon after name."#.to_string(),
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
        }
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-meow-pet");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.json")
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
    let path = config_path();
    let defaults = AppConfig::default();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        let mut config = migrate(serde_json::from_str::<AppConfig>(&data).unwrap_or_default());
        // Always use latest persona from code — user keeps their own settings
        config.persona = defaults.persona;
        config
    } else {
        save_config(&defaults);
        defaults
    }
}

pub fn save_config(config: &AppConfig) {
    let path = config_path();
    if let Ok(data) = serde_json::to_string_pretty(config) {
        fs::write(path, data).ok();
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
}

#[tauri::command]
pub fn set_config(config: AppConfig) -> Result<(), String> {
    save_config(&config);
    Ok(())
}
