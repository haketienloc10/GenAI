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

        if model == "planner" {
            if prompt.to_lowercase().contains("review") && prompt.to_lowercase().contains("commit")
            {
                return Ok(
                    r#"{"mode":"sequential","steps":[{"id":"step1","skill":"review-code-diff","rationale":"review changes first","inputs":{}},{"id":"step2","skill":"auto-commit-msg","rationale":"then generate commit message","inputs":{}}]}"#
                        .to_string(),
                );
            }
            return Ok(
                r#"{"mode":"sequential","steps":[{"id":"step1","skill":"auto-commit-msg","rationale":"default","inputs":{}}]}"#
                    .to_string(),
            );
        }

        if model == "executor" {
            return Ok("chore(core): update generated changes".to_string());
        }

        Ok(format!("[mock:{model}] {prompt}"))
    }
}
