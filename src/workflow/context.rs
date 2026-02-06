use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ExecutionContext {
    vars: HashMap<String, String>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }

    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.vars
    }
}
