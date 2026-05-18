use crate::ai::send_to_bedrock;
use crate::config::{load_config, AppConfig};

#[tauri::command]
pub async fn chat_message(user_text: String) -> Result<String, String> {
    let config = load_config();

    let chat_persona = format!(
        "{}\nThe user is now chatting with you directly. Reply naturally in 1-2 short sentences. Stay in character.",
        config.persona
    );

    let chat_config = AppConfig {
        persona: chat_persona,
        ..config
    };

    let user_prompt = format!("User says: {}", user_text);

    send_to_bedrock(&chat_config, &user_prompt, 200, 0.8).await
}
