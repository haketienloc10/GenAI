use crate::skill::model::Skill;

pub fn build_selector_prompt(user_input: &str, skills: &[Skill]) -> String {
    let skills_text = skills
        .iter()
        .map(|s| {
            format!(
                "- name: {}\n  description: {}\n  category: {}\n  tags: {:?}",
                s.metadata.name, s.metadata.description, s.metadata.category, s.metadata.tags
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Select best skill for user request. Return strict JSON: \
{{\"skill\":\"...\",\"confidence\":0.0,\"reason\":\"...\"}}\n\
User input: {user_input}\nAvailable skills:\n{skills_text}"
    )
}

pub fn build_planner_prompt(user_input: &str, skills: &[Skill]) -> String {
    let skills_json = skills
        .iter()
        .map(|skill| {
            format!(
                "{{\"name\":\"{}\",\"description\":\"{}\",\"category\":\"{}\",\"tags\":{},\"capabilities\":{{\"requires_repo\":{},\"supports_interactive\":{}}}}}",
                escape_json_string(&skill.metadata.name),
                escape_json_string(&skill.metadata.description),
                escape_json_string(&skill.metadata.category),
                serde_json::to_string(&skill.metadata.tags).unwrap_or_else(|_| "[]".to_string()),
                skill.metadata.capabilities.requires_repo,
                skill.metadata.capabilities.supports_interactive,
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "You are a deterministic planner. Temperature is 0.\n\
Given a user input and available skills metadata, return ONLY strict JSON with this schema:\n\
{{\n  \"mode\": \"sequential\" | \"parallel\",\n  \"steps\": [\n    {{\n      \"skill\": \"skill_name\",\n      \"input\": {{}}\n    }}\n  ]\n}}\n\
Rules:\n\
- Use parallel only when skill tasks are independent.\n\
- Use sequential when any dependency exists between steps.\n\
- Steps must reference only skills in the provided list.\n\
- Return no markdown, no explanation, JSON only.\n\
User input: {}\n\
Available skills metadata JSON array: [{}]",
        escape_json_string(user_input),
        skills_json,
    )
}

fn escape_json_string(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}
