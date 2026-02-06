use anyhow::Result;

use crate::llm::client::LlmClient;

pub struct MockLlmClient;

impl MockLlmClient {
    pub fn new() -> Self {
        Self
    }
}

impl LlmClient for MockLlmClient {
    fn generate(&self, model: &str, prompt: &str) -> Result<String> {
        if model == "selector" {
            if prompt.to_lowercase().contains("commit") {
                return Ok(
                    r#"{"skill":"auto-commit-msg","confidence":0.92,"reason":"commit related request"}"#
                        .to_string(),
                );
            }
            return Ok(
                r#"{"skill":"auto-commit-msg","confidence":0.51,"reason":"default"}"#.to_string(),
            );
        }

        if model == "executor" {
            return Ok("chore(core): update generated changes".to_string());
        }

        Ok(format!("[mock:{model}] {prompt}"))
    }
}
