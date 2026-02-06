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
