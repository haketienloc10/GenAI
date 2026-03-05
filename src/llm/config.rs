use anyhow::Result;

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub openai_base_url: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
}

impl LlmConfig {
    pub fn from_env() -> Result<Self> {
        let openai_base_url = std::env::var("OPENAI_BASE_URL")
            .or_else(|_| std::env::var("OPENROUTERLOCAL_BASE_URL"))
            .unwrap_or_else(|_| "http://127.0.0.1:18790/v1".to_string());
        let openai_api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENROUTERLOCAL_API_KEY"))
            .ok();
        let openai_model = std::env::var("OPENAI_MODEL")
            .or_else(|_| std::env::var("OPENROUTERLOCAL_MODEL"))
            .unwrap_or_else(|_| "qwen-cli".to_string());

        Ok(Self {
            openai_base_url,
            openai_api_key,
            openai_model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::LlmConfig;

    #[test]
    fn load_config_from_env() {
        unsafe {
            std::env::set_var("OPENAI_BASE_URL", "http://localhost:9999/v1");
            std::env::set_var("OPENAI_API_KEY", "openai-key");
            std::env::set_var("OPENAI_MODEL", "local-model");
        }

        let cfg = LlmConfig::from_env().expect("expected config from env");
        assert_eq!(cfg.openai_base_url, "http://localhost:9999/v1");
        assert_eq!(cfg.openai_api_key.as_deref(), Some("openai-key"));
        assert_eq!(cfg.openai_model, "local-model");
    }
}
