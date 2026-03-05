use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub openai_api_key: String,
    pub openai_model: String,
    pub openai_base_url: String,
}

impl LlmConfig {
    pub fn from_env() -> Result<Self> {
        let openai_api_key = std::env::var("OPENAI_API_KEY")
            .context("Missing OPENAI_API_KEY in environment or .env")?;
        let openai_model =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let openai_base_url =
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".to_string());

        Ok(Self {
            openai_api_key,
            openai_model,
            openai_base_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::LlmConfig;

    #[test]
    fn load_config_from_env() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "test-key");
            std::env::set_var("OPENAI_MODEL", "test-model");
            std::env::set_var("OPENAI_BASE_URL", "https://example.com");
        }

        let cfg = LlmConfig::from_env().expect("expected config from env");
        assert_eq!(cfg.openai_api_key, "test-key");
        assert_eq!(cfg.openai_model, "test-model");
        assert_eq!(cfg.openai_base_url, "https://example.com");
    }
}
