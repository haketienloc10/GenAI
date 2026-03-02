use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::task::JoinHandle;
use tracing::info;

use crate::orchestrator::planner::{ExecutionMode, ExecutionPlan};
use crate::skill::model::Skill;
use crate::workflow::executor::{ExecutionInput, SkillResult, WorkflowExecutor};

#[derive(Debug, Clone, Default)]
pub struct GlobalExecutionContext {
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PlanExecutionResult {
    pub outputs: Vec<(String, SkillResult)>,
    pub global_context: GlobalExecutionContext,
}

impl PlanExecutionResult {
    pub fn combined_output(&self) -> String {
        self.outputs
            .iter()
            .enumerate()
            .map(|(index, (skill_name, result))| {
                format!(
                    "## Step {}: {}\n{}",
                    index + 1,
                    skill_name,
                    result.output.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

type LlmFactory = Arc<dyn Fn() -> Box<dyn crate::llm::client::LlmClient> + Send + Sync>;

pub struct OrchestratorExecutor {
    skills_by_name: HashMap<String, Skill>,
    llm_factory: LlmFactory,
}

impl OrchestratorExecutor {
    pub fn new(skills: &[Skill], llm_factory: LlmFactory) -> Self {
        let skills_by_name = skills
            .iter()
            .cloned()
            .map(|skill| (skill.metadata.name.clone(), skill))
            .collect::<HashMap<_, _>>();

        Self {
            skills_by_name,
            llm_factory,
        }
    }

    pub async fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        user_prompt: &str,
        debug: bool,
        initial_context: GlobalExecutionContext,
    ) -> Result<PlanExecutionResult> {
        info!("Planning execution...");

        let effective_mode = self.enforce_mode(plan);
        info!("Plan mode: {}", mode_as_str(&effective_mode));

        match effective_mode {
            ExecutionMode::Sequential => {
                self.execute_sequential(plan, user_prompt, debug, initial_context)
                    .await
            }
            ExecutionMode::Parallel => {
                self.execute_parallel(plan, user_prompt, debug, initial_context)
                    .await
            }
        }
    }

    fn enforce_mode(&self, plan: &ExecutionPlan) -> ExecutionMode {
        if matches!(plan.mode, ExecutionMode::Sequential) {
            return ExecutionMode::Sequential;
        }

        let all_safe_parallel = plan.steps.iter().all(|step| {
            self.skills_by_name
                .get(&step.skill_name)
                .map(|skill| {
                    let permissions = &skill.metadata.permissions;
                    permissions.run_commands && !permissions.write_access
                })
                .unwrap_or(false)
        });

        if all_safe_parallel {
            ExecutionMode::Parallel
        } else {
            tracing::warn!(
                "Parallel plan downgraded to sequential due to permission safety policy"
            );
            ExecutionMode::Sequential
        }
    }

    async fn execute_sequential(
        &self,
        plan: &ExecutionPlan,
        user_prompt: &str,
        debug: bool,
        mut global_context: GlobalExecutionContext,
    ) -> Result<PlanExecutionResult> {
        let mut outputs = Vec::with_capacity(plan.steps.len());

        let total_steps = plan.steps.len();
        for (index, step) in plan.steps.iter().enumerate() {
            let skill = self.skill_or_err(&step.skill_name)?;
            info!(
                "Executing step {}/{}: {}",
                index + 1,
                total_steps,
                step.skill_name
            );

            let mut vars = global_context.variables.clone();
            merge_json_input(&mut vars, &step.inputs);

            let skill_result = execute_skill_with_factory(
                self.llm_factory.clone(),
                skill.clone(),
                ExecutionInput {
                    user_prompt: user_prompt.to_string(),
                    debug,
                    variables: vars,
                },
            )
            .await?;

            global_context
                .variables
                .extend(skill_result.context.clone());
            outputs.push((step.skill_name.clone(), skill_result));
        }

        Ok(PlanExecutionResult {
            outputs,
            global_context,
        })
    }

    async fn execute_parallel(
        &self,
        plan: &ExecutionPlan,
        user_prompt: &str,
        debug: bool,
        global_context: GlobalExecutionContext,
    ) -> Result<PlanExecutionResult> {
        let mut handles: Vec<JoinHandle<Result<(String, SkillResult)>>> = Vec::new();

        for step in &plan.steps {
            let skill = self.skill_or_err(&step.skill_name)?.clone();
            let llm_factory = self.llm_factory.clone();
            let skill_name = step.skill_name.clone();
            let mut vars = global_context.variables.clone();
            merge_json_input(&mut vars, &step.inputs);
            let prompt = user_prompt.to_string();

            info!("Executing parallel skill: {}", skill_name);
            let handle = tokio::spawn(async move {
                let result = execute_skill_with_factory(
                    llm_factory,
                    skill,
                    ExecutionInput {
                        user_prompt: prompt,
                        debug,
                        variables: vars,
                    },
                )
                .await?;

                Ok((skill_name, result))
            });
            handles.push(handle);
        }

        let mut outputs = Vec::new();
        let mut errors = Vec::new();
        let mut merged_context = global_context;

        for handle in handles {
            match handle.await {
                Ok(Ok((skill_name, result))) => {
                    merged_context.variables.extend(result.context.clone());
                    outputs.push((skill_name, result));
                }
                Ok(Err(err)) => errors.push(err.to_string()),
                Err(join_err) => errors.push(join_err.to_string()),
            }
        }

        if errors.is_empty() {
            Ok(PlanExecutionResult {
                outputs,
                global_context: merged_context,
            })
        } else {
            Err(anyhow!(
                "One or more parallel skills failed:\n{}",
                errors.join("\n")
            ))
        }
    }

    fn skill_or_err(&self, skill_name: &str) -> Result<&Skill> {
        self.skills_by_name
            .get(skill_name)
            .ok_or_else(|| anyhow!("Skill not found in plan: {skill_name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_output_uses_step_headers() {
        let result = PlanExecutionResult {
            outputs: vec![
                (
                    "review-code-diff".to_string(),
                    SkillResult {
                        output: "Nhận xét".to_string(),
                        context: HashMap::new(),
                    },
                ),
                (
                    "auto-commit-msg".to_string(),
                    SkillResult {
                        output: "fix(core): improve planner".to_string(),
                        context: HashMap::new(),
                    },
                ),
            ],
            global_context: GlobalExecutionContext::default(),
        };

        assert_eq!(
            result.combined_output(),
            "## Step 1: review-code-diff\nNhận xét\n\n## Step 2: auto-commit-msg\nfix(core): improve planner"
        );
    }
}

async fn execute_skill_with_factory(
    llm_factory: LlmFactory,
    skill: Skill,
    input: ExecutionInput,
) -> Result<SkillResult> {
    tokio::task::spawn_blocking(move || {
        let mut executor = WorkflowExecutor::new((llm_factory)());
        executor.execute_skill(&skill, input)
    })
    .await
    .map_err(|err| anyhow!("Skill task failed to join: {err}"))?
}

fn merge_json_input(vars: &mut HashMap<String, String>, input: &serde_json::Value) {
    if let Some(obj) = input.as_object() {
        for (key, value) in obj {
            vars.insert(key.clone(), value_to_string(value));
        }
    }
}

fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

fn mode_as_str(mode: &ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Sequential => "sequential",
        ExecutionMode::Parallel => "parallel",
    }
}
