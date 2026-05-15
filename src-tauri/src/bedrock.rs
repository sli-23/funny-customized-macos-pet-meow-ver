use crate::config::load_config;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, InferenceConfiguration, Message, SystemContentBlock,
};
use aws_sdk_bedrockruntime::Client;

#[tauri::command]
pub async fn generate_message(context: String) -> Result<String, String> {
    let config = load_config();

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(config.aws_region.clone()))
        .load()
        .await;

    let client = Client::new(&aws_config);

    let hour = chrono_hour();
    let time_context = if hour >= 18 {
        "It's evening/night. Remind them to stop working and go home."
    } else if hour < 9 {
        "It's early morning."
    } else {
        "It's during work hours."
    };

    let user_prompt = format!(
        "Current activity: {}\n{}\nGenerate ONE short message (under 40 chars):",
        context, time_context
    );

    let inference_config = InferenceConfiguration::builder()
        .max_tokens(60)
        .temperature(0.9_f32)
        .build();

    let result = client
        .converse()
        .model_id(&config.model_id)
        .system(SystemContentBlock::Text(config.persona.clone()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(user_prompt))
                .build()
                .map_err(|e| e.to_string())?,
        )
        .inference_config(inference_config)
        .send()
        .await;

    match result {
        Ok(response) => {
            if let Some(output) = response.output() {
                if let aws_sdk_bedrockruntime::types::ConverseOutput::Message(msg) = output {
                    for block in msg.content() {
                        if let ContentBlock::Text(text) = block {
                            let clean = text.trim().trim_matches('"').to_string();
                            return Ok(clean);
                        }
                    }
                }
            }
            Err("No response from model".to_string())
        }
        Err(e) => Err(format!("Bedrock error: {}", e)),
    }
}

fn chrono_hour() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // rough hour calculation (UTC offset for PST = -7, adjust as needed)
    ((secs / 3600) % 24) as u32
}
