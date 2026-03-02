use std::collections::HashMap;

use anyhow::Result;

use crate::llm::client::LlmClient;
use crate::skill::model::Skill;
use crate::workflow::condition::evaluate_if;
use crate::workflow::context::ExecutionContext;
use crate::workflow::step::execute_step;

#[derive(Debug, Clone, Default)]
pub struct ExecutionInput {
    pub user_prompt: String,
    pub debug: bool,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillResult {
    pub output: String,
    pub context: HashMap<String, String>,
}

pub struct WorkflowExecutor {
    llm: Box<dyn LlmClient>,
}

impl WorkflowExecutor {
    pub fn new(llm: Box<dyn LlmClient>) -> Self {
        Self { llm }
    }

    pub fn execute_skill(&mut self, skill: &Skill, input: ExecutionInput) -> Result<SkillResult> {
        let mut ctx = ExecutionContext::new();
        ctx.set("user_input", input.user_prompt);
        ctx.set("debug", input.debug.to_string());

        for (key, value) in input.variables {
            ctx.set(key, value);
        }

        let mut final_output = String::new();

        for step in &skill.steps {
            if let Some(expr) = &step.if_expr {
                if !evaluate_if(expr, &ctx)? {
                    continue;
                }
            }

            if let Some(out) = execute_step(step, &mut ctx, self.llm.as_ref())? {
                final_output = out;
            }
        }

        Ok(SkillResult {
            output: final_output,
            context: ctx.as_map().clone(),
        })
    }
}
