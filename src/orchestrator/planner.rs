use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llm::client::LlmClient;
use crate::llm::prompt::{build_planner_prompt, build_planner_repair_prompt};
use crate::skill::model::Skill;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Sequential,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedSkill {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "skill")]
    pub skill_name: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default, rename = "inputs", alias = "input")]
    pub inputs: Value,
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

        let prompt = build_planner_prompt(user_input, skills);
        let raw = self
            .llm
            .generate("planner", &prompt)
            .map_err(|err| anyhow!("Planner LLM request failed: {err}"))?;

        match parse_and_validate_plan(&raw, skills) {
            Ok(plan) => Ok(plan),
            Err(validation_err) => {
                tracing::warn!(
                    "Planner returned invalid plan; requesting one correction pass: {validation_err}"
                );
                let repair_prompt = build_planner_repair_prompt(
                    user_input,
                    skills,
                    &raw,
                    &validation_err.to_string(),
                );
                let repaired_raw = self
                    .llm
                    .generate("planner", &repair_prompt)
                    .map_err(|err| anyhow!("Planner correction request failed: {err}"))?;

                parse_and_validate_plan(&repaired_raw, skills).map_err(|repair_err| {
                    anyhow!(
                        "Planner returned invalid plan after retry: {repair_err}. Raw response: {repaired_raw}"
                    )
                })
            }
        }
    }
}

fn parse_and_validate_plan(raw: &str, skills: &[Skill]) -> Result<ExecutionPlan> {
    let plan = parse_plan_json(raw)?;
    validate_plan(plan, skills)
}

fn parse_plan_json(raw: &str) -> Result<ExecutionPlan> {
    serde_json::from_str::<ExecutionPlan>(raw)
        .map_err(|err| anyhow!("Invalid planner JSON: {err}. Raw response: {raw}"))
}

fn validate_plan(mut plan: ExecutionPlan, skills: &[Skill]) -> Result<ExecutionPlan> {
    if plan.steps.is_empty() {
        return Err(anyhow!("Plan steps cannot be empty"));
    }

    for (index, step) in plan.steps.iter_mut().enumerate() {
        if step.id.trim().is_empty() {
            step.id = format!("step{}", index + 1);
        }
        if step.rationale.trim().is_empty() {
            step.rationale = "No rationale provided".to_string();
        }
        if step.skill_name.trim().is_empty() {
            return Err(anyhow!("Plan step {} has empty skill", index + 1));
        }
        if !step.inputs.is_object() {
            return Err(anyhow!(
                "Plan step {} inputs must be a JSON object",
                index + 1
            ));
        }
        if !skills
            .iter()
            .any(|skill| skill.metadata.name == step.skill_name)
        {
            return Err(anyhow!(
                "Plan step {} references unknown skill '{}'.",
                index + 1,
                step.skill_name
            ));
        }
    }

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::model::{Capabilities, Permissions, ResponseFormat, SkillMetadata};
    use std::collections::VecDeque;
    use std::sync::Mutex;

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

    struct QueueLlm {
        responses: Mutex<VecDeque<String>>,
    }

    impl QueueLlm {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Mutex::new(VecDeque::from(responses)),
            }
        }
    }

    impl LlmClient for QueueLlm {
        fn generate(&self, _model: &str, _prompt: &str) -> Result<String> {
            self.responses
                .lock()
                .expect("queue lock")
                .pop_front()
                .ok_or_else(|| anyhow!("No mocked planner response"))
        }
    }

    #[test]
    fn planner_json_parsing_valid_and_invalid() {
        let valid = r#"{"mode":"sequential","steps":[{"id":"step1","skill":"review-code-diff","rationale":"review first","inputs":{}}]}"#;
        assert!(parse_plan_json(valid).is_ok());
        assert!(parse_plan_json("not-json").is_err());
    }

    #[test]
    fn validation_rejects_unknown_skill() {
        let skills = vec![mock_skill("review-code-diff")];
        let plan = ExecutionPlan {
            mode: ExecutionMode::Sequential,
            steps: vec![PlannedSkill {
                id: "step1".to_string(),
                skill_name: "unknown-skill".to_string(),
                rationale: "x".to_string(),
                inputs: Value::Object(Default::default()),
            }],
        };

        assert!(validate_plan(plan, &skills).is_err());
    }

    #[test]
    fn generates_two_step_plan_for_review_and_commit_request() {
        let skills = vec![
            mock_skill("review-code-diff"),
            mock_skill("auto-commit-msg"),
        ];
        let llm = QueueLlm::new(vec![
            r#"{"mode":"sequential","steps":[{"id":"step1","skill":"review-code-diff","rationale":"review","inputs":{}},{"id":"step2","skill":"auto-commit-msg","rationale":"commit","inputs":{}}]}"#.to_string(),
        ]);
        let planner = Planner::new(Box::new(llm));

        let plan = planner
            .generate_plan("review code và tạo message commit", &skills)
            .expect("plan should be generated");

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].skill_name, "review-code-diff");
        assert_eq!(plan.steps[1].skill_name, "auto-commit-msg");
    }

    #[test]
    fn retries_once_when_first_plan_is_invalid() {
        let skills = vec![mock_skill("review-code-diff")];
        let llm = QueueLlm::new(vec![
            "not-json".to_string(),
            r#"{"mode":"sequential","steps":[{"id":"step1","skill":"review-code-diff","rationale":"fixed","inputs":{}}]}"#.to_string(),
        ]);
        let planner = Planner::new(Box::new(llm));

        let plan = planner
            .generate_plan("review", &skills)
            .expect("planner should recover on retry");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].skill_name, "review-code-diff");
    }
}
