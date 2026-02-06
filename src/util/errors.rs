use thiserror::Error;

#[derive(Debug, Error)]
pub enum GenAiError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Execution error: {0}")]
    Execution(String),
}
