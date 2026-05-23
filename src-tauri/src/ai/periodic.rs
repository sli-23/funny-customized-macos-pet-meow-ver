use crate::ai::history::ChatHistory;
use crate::ai::{data_dir, send_to_ai};
use crate::config::load_config;
use crate::runtime::commands::RuntimeState;
use chrono::Timelike;
use tauri::Manager;

pub(crate) fn build_periodic_prompt(
    context: &str,
    time_ctx: &str,
    pet_ctx: &str,
    recent_chat: &str,
) -> String {
    let chat_section = if recent_chat.is_empty() {
        String::new()
    } else {
        format!("\n\nRecent chat with user:\n{}", recent_chat)
    };

    format!(
        "Current context:\n{}\n{}\n\nPet personality context:\n{}{}\n\nRules: React based on the personality context above. If [Mood: cautious], be gentle. If [Streak], mention it. If [typing], comment on typing. If Spotify is playing, comment on the song.\nGenerate ONE short cute cat message (under 40 chars). Be playful like a real cat. NEVER reply with just a single word or sound. Always include context about what they're doing:",
        context, time_ctx, pet_ctx, chat_section
    )
}

#[tauri::command]
pub async fn generate_message(app: tauri::AppHandle, context: String) -> Result<String, String> {
    let config = load_config();
    let hour = local_hour();
    let time_context = build_time_context(hour);

    let pet_context = {
        let state = app.state::<RuntimeState>();
        let ctx = state.context_engine.read().await;
        ctx.get_context_string()
    };

    let history = ChatHistory::new(data_dir());
    let recent_chat = history.format_for_prompt(3);

    let user_prompt = build_periodic_prompt(&context, &time_context, &pet_context, &recent_chat);

    send_to_ai(&config, &user_prompt, 60, 0.9).await
}

fn local_hour() -> u32 {
    chrono::Local::now().hour()
}

fn build_time_context(hour: u32) -> String {
    match hour {
        0..=5 => "It's very late at night. They should be sleeping!".to_string(),
        6..=8 => "It's early morning.".to_string(),
        9..=11 => "It's morning work hours.".to_string(),
        12..=13 => "It's lunch time! Remind them to eat and take a break.".to_string(),
        14..=17 => "It's afternoon work hours. Remind them to drink water and sit up straight.".to_string(),
        18..=19 => "It's evening. They should stop working and go home!".to_string(),
        20..=23 => "It's night time. If still working, tell them to rest.".to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Checkpoint 4: build_periodic_prompt (pure) ────────────────────────────

    #[test]
    fn test_periodic_prompt_no_chat_section_when_history_empty() {
        let p = build_periodic_prompt("ctx", "morning", "happy", "");
        assert!(!p.contains("Recent chat"));
    }

    #[test]
    fn test_periodic_prompt_includes_recent_chat_when_present() {
        let chat = "[Recent conversation]\nYou: hi\nClaudeMeow: meow";
        let p = build_periodic_prompt("ctx", "morning", "happy", chat);
        assert!(p.contains("Recent chat with user:"));
        assert!(p.contains("You: hi"));
    }

    #[test]
    fn test_periodic_prompt_includes_context_and_time() {
        let p = build_periodic_prompt("VS Code open", "lunch time", "coding streak", "");
        assert!(p.contains("VS Code open"));
        assert!(p.contains("lunch time"));
        assert!(p.contains("coding streak"));
    }
}
