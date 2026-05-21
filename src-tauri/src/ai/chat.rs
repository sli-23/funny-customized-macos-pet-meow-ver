use crate::ai::send_to_bedrock;
use crate::config::{load_config, AppConfig};
use crate::modules;

#[tauri::command]
pub async fn chat_message(user_text: String) -> Result<String, String> {
    let config = load_config();

    let context = modules::get_activity_context();
    let weather = modules::weather::get_context();

    let chat_persona = format!(
        "{}\nThe user is now chatting with you directly. Reply naturally in 1-2 short sentences. Stay in character.\nContext you know: {}\n{}\n\nAbout yourself (if the user asks what you can do, your features, or about yourself):\n- You are ClaudeMeow, a desktop pet cat app\n- You can detect which app the user is using and react to it\n- You react to: coding (VS Code, Terminal, IntelliJ), Slack, Zoom/Chime meetings, browser URLs (GitHub, YouTube, Reddit), Spotify\n- You have a module system: users can drop module folders to add new reactions\n- You track pet status: happiness, energy, love\n- You remind users about health: posture, water, Monster drinks, breaks\n- You have a dev console for debugging\n- Users can chat with you using Ctrl+Cmd+C\n- You show floating +3/-15 score numbers when stats change\n- You support .meow secret files from friends\n- You are powered by Claude AI via Amazon Bedrock",
        config.persona, context, weather
    );

    let chat_config = AppConfig {
        persona: chat_persona,
        ..config
    };

    let user_prompt = format!("User says: {}", user_text);

    send_to_bedrock(&chat_config, &user_prompt, 200, 0.8).await
}
