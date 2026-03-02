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
    let skills_catalog = build_skills_catalog(skills);

    format!(
        "SYSTEM:\n\
You are a planning engine for a developer agent. Your job is to produce a minimal plan using available skills.\n\n\
USER:\n\
User request:\n\
{}\n\n\
Available skills (do not invent new skills):\n\
{}\n\n\
Rules:\n\
- Output MUST be valid JSON ONLY. No markdown. No extra text.\n\
- You MUST output a plan with 1 or more steps.\n\
- Each step MUST reference an existing skill by exact `name`.\n\
- If the request asks for multiple tasks, include multiple skills.\n\
- Prefer sequential mode unless steps are independent.\n\
- When asked to review code AND create a commit message, you MUST include these steps in order:\n\
  1. review-code-diff\n\
  2. auto-commit-msg\n\n\
Return JSON with this schema:\n\
{{\n\
  \"mode\": \"sequential\",\n\
  \"steps\": [\n\
    {{ \"id\": \"step1\", \"skill\": \"<skill_name>\", \"rationale\": \"<short>\", \"inputs\": {{}} }}\n\
  ]\n\
}}",
        user_input, skills_catalog
    )
}

pub fn build_planner_repair_prompt(
    user_input: &str,
    skills: &[Skill],
    invalid_json: &str,
    validation_error: &str,
) -> String {
    let skills_catalog = build_skills_catalog(skills);

    format!(
        "Your previous planner response was invalid. Return corrected JSON only.\n\
User request:\n{}\n\n\
Available skills:\n{}\n\n\
Validation error:\n{}\n\n\
Invalid response:\n{}\n\n\
Return corrected JSON matching schema:\n\
{{\"mode\":\"sequential\",\"steps\":[{{\"id\":\"step1\",\"skill\":\"<skill_name>\",\"rationale\":\"<short>\",\"inputs\":{{}}}}]}}",
        user_input, skills_catalog, validation_error, invalid_json
    )
}

fn build_skills_catalog(skills: &[Skill]) -> String {
    skills
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
        .join(",")
}

fn escape_json_string(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}
