pub mod chat;
pub mod periodic;

use crate::config::AppConfig;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, InferenceConfiguration, Message, SystemContentBlock,
};
use aws_sdk_bedrockruntime::Client;
use serde_json::{json, Value};

pub async fn send_to_bedrock(
    config: &AppConfig,
    user_prompt: &str,
    max_tokens: i32,
    temperature: f32,
) -> Result<String, String> {
    let mut config = config.clone();
    if !config.user_nickname.is_empty() {
        let persona = config.persona
            .lines()
            .filter(|line| !line.contains("Call the user by nickname"))
            .collect::<Vec<_>>()
            .join("\n");
        config.persona = format!(
            "{}\nThe user's name is '{}'. ALWAYS call them '{}' and NOTHING else. Never use any other nickname or generic term.",
            persona, config.user_nickname, config.user_nickname
        );
    }
    match config.auth_mode.as_str() {
        "apikey" => send_via_bearer(&config, user_prompt, max_tokens, temperature).await,
        _ => send_via_iam(&config, user_prompt, max_tokens, temperature).await,
    }
}

#[tauri::command]
pub async fn test_api(api_key: String, region: String, model_id: String) -> Result<String, String> {
    let url = format!(
        "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
        region, model_id
    );

    let body = json!({
        "system": [{"text": "Reply with exactly: API connection successful!"}],
        "messages": [{"role": "user", "content": [{"text": "test"}]}],
        "inferenceConfig": {"maxTokens": 20, "temperature": 0.0}
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    Ok("API connection successful!".to_string())
}

async fn send_via_iam(
    config: &AppConfig,
    user_prompt: &str,
    max_tokens: i32,
    temperature: f32,
) -> Result<String, String> {
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(config.aws_region.clone()))
        .load()
        .await;

    let client = Client::new(&aws_config);

    let inference_config = InferenceConfiguration::builder()
        .max_tokens(max_tokens)
        .temperature(temperature)
        .build();

    let result = client
        .converse()
        .model_id(&config.model_id)
        .system(SystemContentBlock::Text(config.persona.clone()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(user_prompt.to_string()))
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
                            return Ok(text.trim().trim_matches('"').to_string());
                        }
                    }
                }
            }
            Err("No response from model".to_string())
        }
        Err(e) => Err(format!("Bedrock error: {}", e)),
    }
}

async fn send_via_bearer(
    config: &AppConfig,
    user_prompt: &str,
    max_tokens: i32,
    temperature: f32,
) -> Result<String, String> {
    let url = format!(
        "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
        config.aws_region, config.model_id
    );

    let body = json!({
        "system": [{"text": config.persona}],
        "messages": [{
            "role": "user",
            "content": [{"text": user_prompt}]
        }],
        "inferenceConfig": {
            "maxTokens": max_tokens,
            "temperature": temperature
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("API {}: {}", status, text));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    json.get("output")
        .and_then(|o| o.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.trim().trim_matches('"').to_string())
        .ok_or_else(|| "No text in response".to_string())
}
