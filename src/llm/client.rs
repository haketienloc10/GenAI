use anyhow::Result;

pub trait LlmClient: Send + Sync {
    fn generate(&self, model: &str, prompt: &str) -> Result<String>;
}
