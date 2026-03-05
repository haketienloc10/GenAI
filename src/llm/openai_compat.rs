use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::llm::client::LlmClient;
use crate::llm::config::LlmConfig;
use crate::llm::mock::MockLlmClient;

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

pub struct OpenAiCompatLlmClient {
    http: Client,
    config: LlmConfig,
    fallback: MockLlmClient,
}

impl OpenAiCompatLlmClient {
    pub fn new(config: LlmConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self {
            http,
            config,
            fallback: MockLlmClient::new(),
        })
    }

    fn build_request(model: &str, prompt: &str) -> OpenAiChatRequest {
        OpenAiChatRequest {
            model: model.to_string(),
            messages: vec![OpenAiMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        }
    }

    fn parse_response(raw: &str) -> Result<String> {
        let parsed: OpenAiChatResponse = serde_json::from_str(raw)
            .context("Failed to deserialize OpenAI-compatible response")?;

        parsed
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| anyhow!("OpenAI-compatible response has no choices/message/content"))
    }
}

impl LlmClient for OpenAiCompatLlmClient {
    fn generate(&self, model: &str, prompt: &str) -> Result<String> {
        if model == "executor" {
            debug!("Model executor is configured to fallback to mock response");
            return self.fallback.generate(model, prompt);
        }

        let effective_model = if model.trim().is_empty() {
            self.config.openai_model.as_str()
        } else {
            model
        };

        let request_body = Self::build_request(effective_model, prompt);
        let url = format!(
            "{}/chat/completions",
            self.config.openai_base_url.trim_end_matches('/')
        );

        debug!(
            model = effective_model,
            base_url = self.config.openai_base_url,
            "Sending request to OpenAI-compatible gateway"
        );

        let mut request = self
            .http
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request_body);

        if let Some(api_key) = &self.config.openai_api_key {
            request = request.bearer_auth(api_key);
        }

        let response = match request.send() {
            Ok(response) => response,
            Err(err) => {
                warn!("OpenAI-compatible request failed, falling back to mock response: {err}");
                return self.fallback.generate(model, prompt);
            }
        };

        let status = response.status();
        let body = response
            .text()
            .context("Failed to read OpenAI-compatible response body")?;

        if !status.is_success() {
            return Err(anyhow!(
                "OpenAI-compatible gateway request failed (status: {status}): {body}"
            ));
        }

        Self::parse_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::OpenAiCompatLlmClient;

    #[test]
    fn request_builder_matches_chat_completions_shape() {
        let request = OpenAiCompatLlmClient::build_request("qwen-cli", "hello");
        let value = serde_json::to_value(&request).expect("request should be serializable");

        assert_eq!(value["model"], "qwen-cli");
        assert_eq!(value["messages"][0]["role"], "user");
        assert_eq!(value["messages"][0]["content"], "hello");
    }

    #[test]
    fn parser_extracts_first_choice_message_text() {
        let raw = r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": "response text"
              }
            }
          ]
        }"#;

        let parsed = OpenAiCompatLlmClient::parse_response(raw).expect("response should parse");
        assert_eq!(parsed, "response text");
    }
}
