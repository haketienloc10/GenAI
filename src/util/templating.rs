use std::collections::HashMap;

use anyhow::Result;
use regex::Regex;

pub fn render_template(input: &str, context: &HashMap<String, String>) -> Result<String> {
    let re = Regex::new(r"\{\{\s*([a-zA-Z0-9_\-]+)\s*\}\}")?;
    let rendered = re.replace_all(input, |caps: &regex::Captures| {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        context.get(key).cloned().unwrap_or_default()
    });
    Ok(rendered.into_owned())
}
