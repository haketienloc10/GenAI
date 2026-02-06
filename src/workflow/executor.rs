use anyhow::Result;

use crate::llm::client::LlmClient;
use crate::skill::model::Skill;
use crate::workflow::condition::evaluate_if;
use crate::workflow::context::ExecutionContext;
use crate::workflow::step::execute_step;

pub struct ExecutionInput {
    pub user_prompt: String,
    pub debug: bool,
}

pub struct WorkflowExecutor {
    llm: Box<dyn LlmClient>,
}

impl WorkflowExecutor {
    pub fn new(llm: Box<dyn LlmClient>) -> Self {
        Self { llm }
    }

    pub fn execute(&mut self, skill: &Skill, input: ExecutionInput) -> Result<String> {
        let mut ctx = ExecutionContext::new();
        ctx.set("user_input", input.user_prompt);
        ctx.set("debug", input.debug.to_string());

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

        Ok(final_output)
    }
}
