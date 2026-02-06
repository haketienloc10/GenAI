use anyhow::{anyhow, Context, Result};
use regex::Regex;

use crate::skill::model::{SkillMetadata, WorkflowStep};

pub fn parse_frontmatter(content: &str) -> Result<(SkillMetadata, String)> {
    let mut parts = content.splitn(3, "---");
    let prefix = parts.next().unwrap_or_default();
    if !prefix.trim().is_empty() {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter"));
    }

    let yaml = parts
        .next()
        .ok_or_else(|| anyhow!("Missing YAML frontmatter"))?;
    let body = parts
        .next()
        .ok_or_else(|| anyhow!("Missing markdown body after frontmatter"))?;

    let metadata: SkillMetadata =
        serde_yaml::from_str(yaml).context("Failed to parse frontmatter YAML")?;
    Ok((metadata, body.trim_start().to_string()))
}

pub fn parse_genai_steps(markdown_body: &str) -> Result<Vec<WorkflowStep>> {
    let re = Regex::new(r"(?s)```genai-step\s*(.*?)\s*```")?;
    let mut steps = Vec::new();

    for cap in re.captures_iter(markdown_body) {
        let yaml = cap
            .get(1)
            .ok_or_else(|| anyhow!("Invalid genai-step capture"))?
            .as_str();
        let step: WorkflowStep =
            serde_yaml::from_str(yaml).context("Failed parsing genai-step YAML")?;
        steps.push(step);
    }

    Ok(steps)
}
