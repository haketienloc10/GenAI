use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};

use crate::llm::client::LlmClient;
use crate::llm::config::LlmConfig;
use crate::llm::mock::MockLlmClient;

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageResponse,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessageResponse {
    content: String,
}

pub struct OpenAiLlmClient {
    http: Client,
    config: LlmConfig,
    fallback: MockLlmClient,
}

impl OpenAiLlmClient {
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

    fn parse_response(raw: &str) -> Result<String> {
        let parsed: OpenAiResponse =
            serde_json::from_str(raw).context("Failed to deserialize OpenAI response")?;

        parsed
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("OpenAI response has no choices/message/content"))
    }
}

impl LlmClient for OpenAiLlmClient {
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

        let request_body = OpenAiRequest {
            model: effective_model.to_string(),
            messages: vec![OpenAiMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };
        let url = format!(
            "{}/v1/chat/completions",
            self.config.openai_base_url.trim_end_matches('/')
        );

        debug!(model = effective_model, "Sending request to OpenAI-compatible API");

        let response = match self
            .http
            .post(url)
            .bearer_auth(&self.config.openai_api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
        {
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
            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                error!(status = %status, "OpenAI authentication failed");
                return Err(anyhow!(
                    "OpenAI API key invalid or unauthorized (status: {status}): {body}"
                ));
            }

            error!(status = %status, "OpenAI non-success response; falling back to mock");
            return self.fallback.generate(model, prompt);
        }

        Self::parse_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::OpenAiLlmClient;

    #[test]
    fn parser_extracts_first_choice_content() {
        let raw = r#"{
          "choices": [
            {
              "message": {
                "content": "response text"
              }
            }
          ]
        }"#;

        let parsed = OpenAiLlmClient::parse_response(raw).expect("response should parse");
        assert_eq!(parsed, "response text");
    }
}
