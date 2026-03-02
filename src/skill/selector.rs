use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::llm::client::LlmClient;
use crate::llm::prompt::build_selector_prompt;
use crate::skill::model::Skill;

#[derive(Debug, Deserialize)]
struct SelectorResponse {
    skill: String,
    confidence: f64,
    reason: String,
}

pub fn select_skill<'a>(
    user_input: &str,
    skills: &'a [Skill],
    llm: Option<&dyn LlmClient>,
) -> Result<&'a Skill> {
    if skills.is_empty() {
        return Err(anyhow!("No skills found"));
    }

    if let Some(client) = llm {
        let prompt = build_selector_prompt(user_input, skills);
        if let Ok(resp) = client.generate("selector", &prompt) {
            if let Ok(parsed) = serde_json::from_str::<SelectorResponse>(&resp) {
                let _ = (parsed.confidence, &parsed.reason);
                if let Some(skill) = skills.iter().find(|s| s.metadata.name == parsed.skill) {
                    return Ok(skill);
                }
            }
        }
    }

    fallback_select(user_input, skills)
}

fn fallback_select<'a>(user_input: &str, skills: &'a [Skill]) -> Result<&'a Skill> {
    let input = user_input.to_lowercase();

    let scored = skills
        .iter()
        .map(|skill| {
            let mut score = 0usize;
            let mut tag_matches = 0usize;

            if skill.metadata.description.to_lowercase().contains(&input) {
                score += 2;
            }
            if skill.metadata.category.to_lowercase().contains(&input) {
                score += 2;
            }

            for tag in &skill.metadata.tags {
                if input.contains(&tag.to_lowercase()) || tag.to_lowercase().contains(&input) {
                    score += 1;
                    tag_matches += 1;
                }
            }

            (skill, score, tag_matches)
        })
        .max_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));

    scored
        .map(|(skill, _, _)| skill)
        .ok_or_else(|| anyhow!("Unable to select a skill"))
}
