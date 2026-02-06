use anyhow::Result;
use walkdir::WalkDir;

use crate::skill::model::Skill;
use crate::skill::parser::{parse_frontmatter, parse_genai_steps};

pub fn scan_skills(skills_dir: &str) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    for entry in WalkDir::new(skills_dir).min_depth(2).max_depth(2) {
        let entry = entry?;
        if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" {
            continue;
        }

        let content = std::fs::read_to_string(entry.path())?;
        let (metadata, markdown_body) = parse_frontmatter(&content)?;
        let steps = parse_genai_steps(&markdown_body)?;

        skills.push(Skill {
            metadata,
            markdown_body,
            steps,
            path: entry.path().to_string_lossy().to_string(),
        });
    }

    Ok(skills)
}
