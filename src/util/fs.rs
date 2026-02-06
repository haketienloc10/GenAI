use anyhow::Result;

pub fn read_to_string(path: &str) -> Result<String> {
    Ok(std::fs::read_to_string(path)?)
}
