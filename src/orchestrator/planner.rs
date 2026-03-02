use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llm::client::LlmClient;
use crate::llm::prompt::build_planner_prompt;
use crate::skill::model::Skill;
use crate::skill::selector::select_skill;

const REVIEW_SKILL: &str = "review-code-diff";
const COMMIT_SKILL: &str = "auto-commit-msg";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Sequential,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedSkill {
    #[serde(rename = "skill")]
    pub skill_name: String,
    #[serde(default)]
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub mode: ExecutionMode,
    pub steps: Vec<PlannedSkill>,
}

pub struct Planner {
    llm: Box<dyn LlmClient>,
}

impl Planner {
    pub fn new(llm: Box<dyn LlmClient>) -> Self {
        Self { llm }
    }

    pub fn generate_plan(&self, user_input: &str, skills: &[Skill]) -> Result<ExecutionPlan> {
        if skills.is_empty() {
            return Err(anyhow!("No skills found"));
        }

        if let Some(intent_plan) = build_intent_plan(user_input, skills) {
            return Ok(intent_plan);
        }

        let prompt = build_planner_prompt(user_input, skills);
        let raw = match self.llm.generate("planner", &prompt) {
            Ok(raw) => raw,
            Err(err) => {
                tracing::warn!(
                    "Planner LLM request failed; falling back to single-skill selection: {err}"
                );
                return fallback_single_skill(user_input, skills);
            }
        };

        match serde_json::from_str::<ExecutionPlan>(&raw).and_then(validate_plan) {
            Ok(plan) => Ok(ensure_multi_intent_plan(user_input, skills, plan)),
            Err(parse_err) => {
                tracing::warn!(
                    "Planner returned invalid plan JSON; falling back to single-skill selection: {parse_err}"
                );
                fallback_single_skill(user_input, skills)
            }
        }
    }
}

fn validate_plan(plan: ExecutionPlan) -> Result<ExecutionPlan, serde_json::Error> {
    if plan.steps.is_empty() {
        return Err(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Plan steps cannot be empty",
        )));
    }

    if plan
        .steps
        .iter()
        .any(|step| step.skill_name.trim().is_empty() || !step.input.is_object())
    {
        return Err(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Every step requires non-empty skill and object input",
        )));
    }

    Ok(plan)
}

fn fallback_single_skill(user_input: &str, skills: &[Skill]) -> Result<ExecutionPlan> {
    if let Some(intent_plan) = build_intent_plan(user_input, skills) {
        return Ok(intent_plan);
    }

    let fallback = select_skill(user_input, skills, None)?;
    Ok(ExecutionPlan {
        mode: ExecutionMode::Sequential,
        steps: vec![PlannedSkill {
            skill_name: fallback.metadata.name.clone(),
            input: Value::Object(Default::default()),
        }],
    })
}

fn ensure_multi_intent_plan(
    user_input: &str,
    skills: &[Skill],
    plan: ExecutionPlan,
) -> ExecutionPlan {
    match detect_intents(user_input) {
        IntentDetection {
            wants_review: true,
            wants_commit: true,
        } if plan.steps.len() <= 1 => {
            if let Some(intent_plan) = build_intent_plan(user_input, skills) {
                tracing::info!(
                    "Planner returned single skill but both intents were detected; overriding with deterministic two-step plan"
                );
                return intent_plan;
            }
            plan
        }
        _ => plan,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntentDetection {
    wants_review: bool,
    wants_commit: bool,
}

fn detect_intents(user_input: &str) -> IntentDetection {
    let input = user_input.to_lowercase();

    let review_terms = [
        "review",
        "code review",
        "soát",
        "đánh giá",
        "nhận xét",
        "review code",
        "review diff",
    ];
    let commit_terms = [
        "commit",
        "message commit",
        "commit msg",
        "conventional commit",
    ];

    IntentDetection {
        wants_review: review_terms.iter().any(|term| input.contains(term)),
        wants_commit: commit_terms.iter().any(|term| input.contains(term)),
    }
}

fn build_intent_plan(user_input: &str, skills: &[Skill]) -> Option<ExecutionPlan> {
    let intents = detect_intents(user_input);
    if !intents.wants_review && !intents.wants_commit {
        return None;
    }

    let has_review_skill = has_skill(skills, REVIEW_SKILL);
    let has_commit_skill = has_skill(skills, COMMIT_SKILL);

    let mut steps = Vec::new();
    if intents.wants_review && has_review_skill {
        steps.push(PlannedSkill {
            skill_name: REVIEW_SKILL.to_string(),
            input: Value::Object(Default::default()),
        });
    }
    if intents.wants_commit && has_commit_skill {
        steps.push(PlannedSkill {
            skill_name: COMMIT_SKILL.to_string(),
            input: Value::Object(Default::default()),
        });
    }

    if steps.is_empty() {
        return None;
    }

    Some(ExecutionPlan {
        mode: ExecutionMode::Sequential,
        steps,
    })
}

fn has_skill(skills: &[Skill], skill_name: &str) -> bool {
    skills.iter().any(|skill| skill.metadata.name == skill_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::model::{Capabilities, Permissions, ResponseFormat, SkillMetadata};

    fn mock_skill(name: &str) -> Skill {
        Skill {
            metadata: SkillMetadata {
                name: name.to_string(),
                description: "desc".to_string(),
                version: "1.0.0".to_string(),
                category: "cat".to_string(),
                tags: vec![],
                entrypoint: "SKILL.md".to_string(),
                workflow_version: 1,
                capabilities: Capabilities {
                    requires_repo: true,
                    supports_interactive: false,
                },
                permissions: Permissions {
                    run_commands: true,
                    allowed_runners: vec![],
                    allowed_paths: vec![],
                    network_access: false,
                    write_access: false,
                },
                response_format: ResponseFormat {
                    format_type: "markdown".to_string(),
                    style: None,
                },
            },
            markdown_body: String::new(),
            steps: vec![],
            path: String::new(),
        }
    }

    #[test]
    fn detects_both_review_and_commit_intents() {
        let intents = detect_intents("review code và tạo message commit");
        assert!(intents.wants_review);
        assert!(intents.wants_commit);
    }

    #[test]
    fn builds_two_step_plan_when_both_intents_are_present() {
        let skills = vec![mock_skill(REVIEW_SKILL), mock_skill(COMMIT_SKILL)];
        let plan = build_intent_plan("review diff rồi tạo commit message", &skills)
            .expect("expected plan");

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].skill_name, REVIEW_SKILL);
        assert_eq!(plan.steps[1].skill_name, COMMIT_SKILL);
    }

    #[test]
    fn builds_single_step_plan_for_commit_only_intent() {
        let skills = vec![mock_skill(REVIEW_SKILL), mock_skill(COMMIT_SKILL)];
        let plan = build_intent_plan("tạo commit message", &skills).expect("expected plan");

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].skill_name, COMMIT_SKILL);
    }
}
