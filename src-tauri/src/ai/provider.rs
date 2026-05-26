use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, InferenceConfiguration, Message, SystemContentBlock,
};
use aws_sdk_bedrockruntime::Client;
use serde_json::Value;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn send(
        &self,
        persona: &str,
        prompt: &str,
        max_tokens: i32,
        temperature: f32,
    ) -> Result<String, String>;
}

// ── Provider structs ─────────────────────────────────────────────────────────

pub struct BedrockBearerProvider {
    pub api_key: String,
    pub region: String,
    pub model_id: String,
}

pub struct BedrockIamProvider {
    pub region: String,
    pub model_id: String,
}

pub struct AnthropicProvider {
    pub api_key: String,
    pub model_id: String,
}

pub struct OpenAiProvider {
    pub api_key: String,
    pub model_id: String,
}

// ── Request body builders (pub(crate) so tests can reach them) ───────────────

impl AnthropicProvider {
    pub(crate) fn build_request_body(
        &self,
        persona: &str,
        prompt: &str,
        max_tokens: i32,
        temperature: f32,
    ) -> Value {
        serde_json::json!({
            "model": self.model_id,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "system": persona,
            "messages": [{"role": "user", "content": prompt}]
        })
    }

    pub(crate) fn parse_response(json: &Value) -> Result<String, String> {
        json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .map(|s| s.trim().to_string())
            .ok_or_else(|| "No text in Anthropic response".to_string())
    }
}

impl OpenAiProvider {
    pub(crate) fn build_request_body(
        &self,
        persona: &str,
        prompt: &str,
        max_tokens: i32,
        temperature: f32,
    ) -> Value {
        serde_json::json!({
            "model": self.model_id,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "messages": [
                {"role": "system", "content": persona},
                {"role": "user",   "content": prompt}
            ]
        })
    }

    pub(crate) fn parse_response(json: &Value) -> Result<String, String> {
        json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .map(|s| s.trim().to_string())
            .ok_or_else(|| "No text in OpenAI response".to_string())
    }
}

// ── AiProvider impls (stubs — will be filled in Checkpoint 3 & 4) ────────────

#[async_trait]
impl AiProvider for BedrockBearerProvider {
    async fn send(
        &self,
        persona: &str,
        prompt: &str,
        max_tokens: i32,
        temperature: f32,
    ) -> Result<String, String> {
        let url = format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
            self.region, self.model_id
        );
        let body = serde_json::json!({
            "system": [{"text": persona}],
            "messages": [{"role": "user", "content": [{"text": prompt}]}],
            "inferenceConfig": {"maxTokens": max_tokens, "temperature": temperature}
        });
        let response = reqwest::Client::new()
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        let json: Value = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;
        json.get("output")
            .and_then(|o| o.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|block| block.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.trim().trim_matches('"').to_string())
            .ok_or_else(|| "No text in Bedrock response".to_string())
    }
}

#[async_trait]
impl AiProvider for BedrockIamProvider {
    async fn send(
        &self,
        persona: &str,
        prompt: &str,
        max_tokens: i32,
        temperature: f32,
    ) -> Result<String, String> {
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        let client = Client::new(&aws_config);
        let inference_config = InferenceConfiguration::builder()
            .max_tokens(max_tokens)
            .temperature(temperature)
            .build();
        let result = client
            .converse()
            .model_id(&self.model_id)
            .system(SystemContentBlock::Text(persona.to_string()))
            .messages(
                Message::builder()
                    .role(ConversationRole::User)
                    .content(ContentBlock::Text(prompt.to_string()))
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
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    async fn send(
        &self,
        persona: &str,
        prompt: &str,
        max_tokens: i32,
        temperature: f32,
    ) -> Result<String, String> {
        let body = self.build_request_body(persona, prompt, max_tokens, temperature);
        let response = reqwest::Client::new()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
        let json: Value = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;
        Self::parse_response(&json)
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn send(
        &self,
        persona: &str,
        prompt: &str,
        max_tokens: i32,
        temperature: f32,
    ) -> Result<String, String> {
        let body = self.build_request_body(persona, prompt, max_tokens, temperature);
        let response = reqwest::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        let json: Value = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;
        Self::parse_response(&json)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase A: trait contract tests (via MockAiProvider) ───────────────────

    #[tokio::test]
    async fn test_mock_provider_returns_expected_string() {
        let mut mock = MockAiProvider::new();
        mock.expect_send()
            .returning(|_, _, _, _| Ok("meow".to_string()));
        let result = mock.send("persona", "prompt", 200, 0.8).await;
        assert_eq!(result.unwrap(), "meow");
    }

    #[tokio::test]
    async fn test_mock_provider_propagates_error() {
        let mut mock = MockAiProvider::new();
        mock.expect_send()
            .returning(|_, _, _, _| Err("boom".to_string()));
        let result = mock.send("persona", "prompt", 200, 0.8).await;
        assert_eq!(result.unwrap_err(), "boom");
    }

    #[tokio::test]
    async fn test_mock_provider_receives_correct_max_tokens() {
        let mut mock = MockAiProvider::new();
        mock.expect_send()
            .withf(|_, _, max_tokens, _| *max_tokens == 200)
            .returning(|_, _, _, _| Ok("ok".to_string()));
        mock.send("p", "q", 200, 0.8).await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_provider_receives_correct_temperature() {
        let mut mock = MockAiProvider::new();
        mock.expect_send()
            .withf(|_, _, _, temp| (*temp - 0.8_f32).abs() < f32::EPSILON)
            .returning(|_, _, _, _| Ok("ok".to_string()));
        mock.send("p", "q", 200, 0.8).await.unwrap();
    }

    // ── Phase D: Anthropic request body builder ───────────────────────────────

    fn anthropic() -> AnthropicProvider {
        AnthropicProvider {
            api_key: "key".to_string(),
            model_id: "claude-haiku-4-5-20251001".to_string(),
        }
    }

    #[test]
    fn test_anthropic_body_model_field() {
        let body = anthropic().build_request_body("sys", "hi", 100, 0.5);
        assert_eq!(body["model"], "claude-haiku-4-5-20251001");
    }

    #[test]
    fn test_anthropic_body_system_contains_persona() {
        let body = anthropic().build_request_body("my-persona", "hi", 100, 0.5);
        assert_eq!(body["system"], "my-persona");
    }

    #[test]
    fn test_anthropic_body_user_message_role() {
        let body = anthropic().build_request_body("sys", "hello", 100, 0.5);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "hello");
    }

    #[test]
    fn test_anthropic_body_max_tokens_and_temperature() {
        let body = anthropic().build_request_body("sys", "hi", 42, 0.3);
        assert_eq!(body["max_tokens"], 42);
        assert!((body["temperature"].as_f64().unwrap() - 0.3).abs() < 1e-6);
    }

    // ── Phase D: OpenAI request body builder ─────────────────────────────────

    fn openai() -> OpenAiProvider {
        OpenAiProvider {
            api_key: "key".to_string(),
            model_id: "gpt-4o-mini".to_string(),
        }
    }

    #[test]
    fn test_openai_body_model_field() {
        let body = openai().build_request_body("sys", "hi", 100, 0.5);
        assert_eq!(body["model"], "gpt-4o-mini");
    }

    #[test]
    fn test_openai_body_system_message_contains_persona() {
        let body = openai().build_request_body("my-persona", "hi", 100, 0.5);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "my-persona");
    }

    #[test]
    fn test_openai_body_user_message_contains_prompt() {
        let body = openai().build_request_body("sys", "hello there", 100, 0.5);
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "hello there");
    }

    #[test]
    fn test_openai_body_max_tokens_and_temperature() {
        let body = openai().build_request_body("sys", "hi", 77, 0.9);
        assert_eq!(body["max_tokens"], 77);
        assert!((body["temperature"].as_f64().unwrap() - 0.9).abs() < 1e-6);
    }

    // ── Phase E: Anthropic response parser ───────────────────────────────────

    #[test]
    fn test_parse_anthropic_response_extracts_text() {
        let json = serde_json::json!({ "content": [{"type": "text", "text": "hello"}] });
        assert_eq!(AnthropicProvider::parse_response(&json).unwrap(), "hello");
    }

    #[test]
    fn test_parse_anthropic_missing_content_returns_err() {
        let json = serde_json::json!({});
        assert!(AnthropicProvider::parse_response(&json).is_err());
    }

    #[test]
    fn test_parse_anthropic_trims_whitespace() {
        let json = serde_json::json!({ "content": [{"text": "  hello  "}] });
        assert_eq!(AnthropicProvider::parse_response(&json).unwrap(), "hello");
    }

    // ── Phase E: OpenAI response parser ──────────────────────────────────────

    #[test]
    fn test_parse_openai_response_extracts_content() {
        let json = serde_json::json!({
            "choices": [{"message": {"content": "meow"}}]
        });
        assert_eq!(OpenAiProvider::parse_response(&json).unwrap(), "meow");
    }

    #[test]
    fn test_parse_openai_missing_choices_returns_err() {
        let json = serde_json::json!({});
        assert!(OpenAiProvider::parse_response(&json).is_err());
    }

    #[test]
    fn test_parse_openai_trims_whitespace() {
        let json = serde_json::json!({
            "choices": [{"message": {"content": "  meow  "}}]
        });
        assert_eq!(OpenAiProvider::parse_response(&json).unwrap(), "meow");
    }
}
