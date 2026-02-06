use anyhow::{anyhow, Result};

use crate::llm::client::LlmClient;
use crate::skill::model::{StepType, WorkflowStep};
use crate::util::templating::render_template;
use crate::workflow::context::ExecutionContext;

pub fn execute_step(
    step: &WorkflowStep,
    ctx: &mut ExecutionContext,
    llm: &dyn LlmClient,
) -> Result<Option<String>> {
    match step.step_type {
        StepType::Command => {
            let runner = step
                .runner
                .as_deref()
                .ok_or_else(|| anyhow!("Command step missing runner"))?;
            let cmd = step
                .cmd
                .as_deref()
                .ok_or_else(|| anyhow!("Command step missing cmd"))?;

            let output = match runner {
                "bash" => std::process::Command::new("bash")
                    .arg("-lc")
                    .arg(cmd)
                    .output()?,
                _ => return Err(anyhow!("Unsupported runner: {runner}")),
            };

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(var) = &step.output_var {
                ctx.set(var, stdout.clone());
            }
            Ok(Some(stdout))
        }
        StepType::Llm => {
            let model = step
                .model
                .as_deref()
                .ok_or_else(|| anyhow!("LLM step missing model"))?;
            let prompt = step
                .prompt
                .as_deref()
                .ok_or_else(|| anyhow!("LLM step missing prompt"))?;
            let rendered_prompt = render_template(prompt, ctx.as_map())?;
            let response = llm.generate(model, &rendered_prompt)?;
            if let Some(var) = &step.output_var {
                ctx.set(var, response.clone());
            }
            Ok(Some(response))
        }
        StepType::Output => {
            let template = step
                .template
                .as_deref()
                .ok_or_else(|| anyhow!("Output step missing template"))?;
            let rendered = render_template(template, ctx.as_map())?;
            if let Some(var) = &step.output_var {
                ctx.set(var, rendered.clone());
            }
            Ok(Some(rendered))
        }
    }
}
