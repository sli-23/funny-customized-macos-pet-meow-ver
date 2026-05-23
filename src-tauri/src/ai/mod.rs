pub mod chat;
pub mod periodic;
pub mod provider;

use crate::config::AppConfig;
use provider::{
    AiProvider, AnthropicProvider, BedrockBearerProvider, BedrockIamProvider, OpenAiProvider,
};
use serde_json::json;

pub fn build_provider(config: &AppConfig) -> Box<dyn AiProvider + Send + Sync> {
    match config.provider.as_str() {
        "anthropic" => Box::new(AnthropicProvider {
            api_key: config.api_key.clone(),
            model_id: config.anthropic_model_id.clone(),
        }),
        "openai" => Box::new(OpenAiProvider {
            api_key: config.openai_api_key.clone(),
            model_id: config.openai_model_id.clone(),
        }),
        "bedrock_iam" => Box::new(BedrockIamProvider {
            region: config.aws_region.clone(),
            model_id: config.model_id.clone(),
        }),
        // "bedrock_apikey" + all legacy values ("apikey", "bedrock", etc.)
        _ => Box::new(BedrockBearerProvider {
            api_key: config.api_key.clone(),
            region: config.aws_region.clone(),
            model_id: config.model_id.clone(),
        }),
    }
}

pub async fn send_to_ai(
    config: &AppConfig,
    user_prompt: &str,
    max_tokens: i32,
    temperature: f32,
) -> Result<String, String> {
    let mut config = config.clone();
    if !config.user_nickname.is_empty() {
        let persona = config
            .persona
            .lines()
            .filter(|line| {
                !line.contains("Call the user by nickname")
                    && !line.contains("hooman")
                    && !line.contains("铲屎官")
            })
            .collect::<Vec<_>>()
            .join("\n");
        config.persona = format!(
            "{}\nThe user's name is '{}'. ALWAYS call them '{}' and NOTHING else. Never use any other name, nickname, or generic term like 人类/hooman/铲屎官/主人/buddy.",
            persona, config.user_nickname, config.user_nickname
        );
    }
    let provider = build_provider(&config);
    provider
        .send(&config.persona, user_prompt, max_tokens, temperature)
        .await
}

#[tauri::command]
pub async fn test_api(
    provider: String,
    api_key: String,
    region: String,
    model_id: String,
) -> Result<String, String> {
    let ping = "Reply with exactly: API connection successful!";

    match provider.as_str() {
        "anthropic" => {
            let body = json!({
                "model": model_id,
                "max_tokens": 20,
                "temperature": 0.0,
                "system": ping,
                "messages": [{"role": "user", "content": "test"}]
            });
            let response = reqwest::Client::new()
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
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
        "openai" => {
            let body = json!({
                "model": model_id,
                "max_tokens": 20,
                "temperature": 0.0,
                "messages": [
                    {"role": "system", "content": ping},
                    {"role": "user", "content": "test"}
                ]
            });
            let response = reqwest::Client::new()
                .post("https://api.openai.com/v1/chat/completions")
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
        // bedrock_apikey or bedrock_iam (IAM has no key to test interactively)
        _ => {
            let url = format!(
                "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
                region, model_id
            );
            let body = json!({
                "system": [{"text": ping}],
                "messages": [{"role": "user", "content": [{"text": "test"}]}],
                "inferenceConfig": {"maxTokens": 20, "temperature": 0.0}
            });
            let response = reqwest::Client::new()
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_provider(p: &str) -> AppConfig {
        AppConfig {
            provider: p.to_string(),
            ..AppConfig::default()
        }
    }

    // Phase C: build_provider factory — verify routing doesn't panic

    #[test]
    fn test_build_provider_default_gives_bedrock_bearer() {
        // default provider is "bedrock_apikey" → BedrockBearerProvider
        let _p = build_provider(&AppConfig::default());
    }

    #[test]
    fn test_build_provider_anthropic_does_not_panic() {
        let _p = build_provider(&config_with_provider("anthropic"));
    }

    #[test]
    fn test_build_provider_openai_does_not_panic() {
        let _p = build_provider(&config_with_provider("openai"));
    }

    #[test]
    fn test_build_provider_bedrock_iam_does_not_panic() {
        let _p = build_provider(&config_with_provider("bedrock_iam"));
    }

    #[test]
    fn test_build_provider_legacy_apikey_does_not_panic() {
        // old auth_mode="apikey" config — routes to _ arm (BedrockBearer)
        let config = AppConfig {
            provider: "bedrock_apikey".to_string(),
            auth_mode: "apikey".to_string(),
            ..AppConfig::default()
        };
        let _p = build_provider(&config);
    }
}
