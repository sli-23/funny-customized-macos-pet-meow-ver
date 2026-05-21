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

pub fn load_config() -> AppConfig {
    let path = config_path();
    let defaults = AppConfig::default();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        let mut config: AppConfig = serde_json::from_str(&data).unwrap_or_default();
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

#[tauri::command]
pub fn set_config(config: AppConfig) -> Result<(), String> {
    save_config(&config);
    Ok(())
}
