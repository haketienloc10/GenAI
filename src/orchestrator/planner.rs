use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llm::client::LlmClient;
use crate::llm::prompt::build_planner_prompt;
use crate::skill::model::Skill;
use crate::skill::selector::select_skill;

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
            Ok(plan) => Ok(plan),
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
    let fallback = select_skill(user_input, skills, None)?;
    Ok(ExecutionPlan {
        mode: ExecutionMode::Sequential,
        steps: vec![PlannedSkill {
            skill_name: fallback.metadata.name.clone(),
            input: Value::Object(Default::default()),
        }],
    })
}
