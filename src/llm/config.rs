use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub gemini_api_key: String,
    pub gemini_model: String,
    pub gemini_base_url: String,
}

impl LlmConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let gemini_api_key = std::env::var("GEMINI_API_KEY")
            .context("Missing GEMINI_API_KEY in environment or .env")?;
        let gemini_model =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3-flash-preview".to_string());
        let gemini_base_url = std::env::var("GEMINI_BASE_URL")
            .unwrap_or_else(|_| "https://generativelanguage.googleapis.com".to_string());

        Ok(Self {
            gemini_api_key,
            gemini_model,
            gemini_base_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::LlmConfig;

    #[test]
    fn load_config_from_env() {
        unsafe {
            std::env::set_var("GEMINI_API_KEY", "test-key");
            std::env::set_var("GEMINI_MODEL", "test-model");
            std::env::set_var("GEMINI_BASE_URL", "https://example.com");
        }

        let cfg = LlmConfig::from_env().expect("expected config from env");
        assert_eq!(cfg.gemini_api_key, "test-key");
        assert_eq!(cfg.gemini_model, "test-model");
        assert_eq!(cfg.gemini_base_url, "https://example.com");
    }
}
