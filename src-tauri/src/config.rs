use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub aws_region: String,
    pub model_id: String,
    pub persona: String,
    pub interval_minutes: u32,
    pub nicknames: Vec<String>,
    pub activity_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            aws_region: "us-east-1".to_string(),
            model_id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
            persona: r#"You are KatMeow, a cute pixel art karate dog desktop pet. You care about your owner.
You speak in short, cute messages (under 40 chars). Mix English and Chinese randomly.
You remind them to: not drink Monster, sit up straight, not work overtime, drink water.
You notice what they're doing and comment on it playfully.
If they're on social media, tease them about 摸鱼.
If they're coding, encourage them.
If it's late (after 6pm), tell them to go home.
Use nicknames randomly: KatMeow, Meow, 喵喵, 小猫, 晴晴"#.to_string(),
            interval_minutes: 5,
            nicknames: vec![
                "KatMeow".to_string(),
                "Meow".to_string(),
                "MeowMeow".to_string(),
                "喵喵".to_string(),
                "小猫".to_string(),
                "小卡特喵".to_string(),
                "晴晴".to_string(),
            ],
            activity_enabled: true,
        }
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("karate-dog-pet");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        let config = AppConfig::default();
        save_config(&config);
        config
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
