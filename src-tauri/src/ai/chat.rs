use crate::ai::history::{ChatHistory, ChatTurn};
use crate::ai::{data_dir, send_to_ai};
use crate::config::{load_config, AppConfig};
use crate::modules;

pub(crate) fn build_chat_prompt(history_block: &str, user_text: &str) -> String {
    if history_block.is_empty() {
        format!("User says: {}", user_text)
    } else {
        format!("{}\n\nUser says: {}", history_block, user_text)
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

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

    let history = ChatHistory::new(data_dir());
    let history_block = history.format_for_prompt(10);
    let user_prompt = build_chat_prompt(&history_block, &user_text);

    let reply = send_to_ai(&chat_config, &user_prompt, 200, 0.8).await?;

    let ts = now_secs();
    history.append(&ChatTurn { ts, role: "user".into(), text: user_text });
    history.append(&ChatTurn { ts, role: "assistant".into(), text: reply.clone() });

    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Checkpoint 3: build_chat_prompt (pure) ────────────────────────────────

    #[test]
    fn test_chat_prompt_no_history_block_when_empty() {
        let p = build_chat_prompt("", "hello");
        assert_eq!(p, "User says: hello");
    }

    #[test]
    fn test_chat_prompt_includes_history_block() {
        let block = "[Recent conversation]\nYou: hi\nClaudeMeow: meow";
        let p = build_chat_prompt(block, "remember?");
        assert!(p.contains("[Recent conversation]"));
        assert!(p.contains("User says: remember?"));
    }

    #[test]
    fn test_chat_prompt_history_comes_before_user_says() {
        let block = "HISTORY";
        let p = build_chat_prompt(block, "msg");
        let history_pos = p.find("HISTORY").unwrap();
        let user_pos = p.find("User says:").unwrap();
        assert!(history_pos < user_pos);
    }
}
