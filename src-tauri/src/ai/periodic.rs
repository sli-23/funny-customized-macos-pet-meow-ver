use crate::ai::send_to_bedrock;
use crate::config::load_config;
use crate::runtime::commands::RuntimeState;
use chrono::Timelike;
use tauri::Manager;

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

    let user_prompt = format!(
        "Current context:\n{}\n{}\n\nPet personality context:\n{}\n\nRules: React based on the personality context above. If [Mood: cautious], be gentle. If [Streak], mention it. If [typing], comment on typing. If Spotify is playing, comment on the song.\nGenerate ONE short cute cat message (under 40 chars). Be playful like a real cat. NEVER reply with just a single word or sound. Always include context about what they're doing:",
        context, time_context, pet_context
    );

    send_to_bedrock(&config, &user_prompt, 60, 0.9).await
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
