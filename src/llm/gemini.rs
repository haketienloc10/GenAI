use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::llm::client::LlmClient;
use crate::llm::config::LlmConfig;
use crate::llm::mock::MockLlmClient;

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiPart>,
}

pub struct GeminiLlmClient {
    http: Client,
    config: LlmConfig,
    fallback: MockLlmClient,
}

impl GeminiLlmClient {
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

    fn build_request(prompt: &str) -> GeminiRequest {
        GeminiRequest {
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
        }
    }

    fn parse_response(raw: &str) -> Result<String> {
        let parsed: GeminiResponse =
            serde_json::from_str(raw).context("Failed to deserialize Gemini response")?;

        parsed
            .candidates
            .first()
            .and_then(|candidate| candidate.content.parts.first())
            .map(|part| part.text.clone())
            .ok_or_else(|| anyhow!("Gemini response has no candidates/parts/text"))
    }
}

impl LlmClient for GeminiLlmClient {
    fn generate(&self, model: &str, prompt: &str) -> Result<String> {
        if model == "executor" {
            debug!("Model executor is configured to fallback to mock response");
            return self.fallback.generate(model, prompt);
        }

        let effective_model = if model.trim().is_empty() {
            self.config.gemini_model.as_str()
        } else {
            model
        };

        let request_body = Self::build_request(prompt);
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.config.gemini_base_url.trim_end_matches('/'),
            effective_model,
            self.config.gemini_api_key
        );

        debug!(model = effective_model, "Sending request to Gemini");

        let response = self
            .http
            .post(url)
            .json(&request_body)
            .send()
            .context("Gemini request failed (network/timeout)")?;

        let status = response.status();
        let body = response
            .text()
            .context("Failed to read Gemini response body")?;

        if !status.is_success() {
            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                error!(status = %status, "Gemini authentication failed");
                return Err(anyhow!(
                    "Gemini API key invalid or unauthorized (status: {status}): {body}"
                ));
            }

            error!(status = %status, "Gemini non-success response");
            return Err(anyhow!("Gemini returned non-200 status {status}: {body}"));
        }

        Self::parse_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::GeminiLlmClient;

    #[test]
    fn request_builder_matches_expected_shape() {
        let request = GeminiLlmClient::build_request("hello");
        let value = serde_json::to_value(&request).expect("request should be serializable");

        assert_eq!(value["contents"][0]["role"], "user");
        assert_eq!(value["contents"][0]["parts"][0]["text"], "hello");
    }

    #[test]
    fn parser_extracts_first_candidate_text() {
        let raw = r#"{
          "candidates": [
            {
              "content": {
                "parts": [
                  { "text": "response text" }
                ]
              }
            }
          ]
        }"#;

        let parsed = GeminiLlmClient::parse_response(raw).expect("response should parse");
        assert_eq!(parsed, "response text");
    }
}
