use std::collections::HashSet;

use anyhow::{anyhow, Result};

use crate::skill::model::{Skill, StepType};

pub fn validate_skill(skill: &Skill) -> Result<()> {
    let metadata = &skill.metadata;

    if metadata.name.trim().is_empty() {
        return Err(anyhow!("Skill name cannot be empty"));
    }
    if metadata.entrypoint != "workflow" {
        return Err(anyhow!("Only entrypoint=workflow is supported"));
    }
    if metadata.workflow_version != 1 {
        return Err(anyhow!("Only workflow_version=1 is supported"));
    }

    let mut ids = HashSet::new();
    let mut output_vars = HashSet::new();

    for step in &skill.steps {
        if !ids.insert(step.id.clone()) {
            return Err(anyhow!("Duplicate step id: {}", step.id));
        }

        if let Some(var) = &step.output_var {
            if !output_vars.insert(var.clone()) {
                return Err(anyhow!("Duplicate output_var: {var}"));
            }
        }

        match step.step_type {
            StepType::Command => {
                if !metadata.permissions.run_commands {
                    return Err(anyhow!(
                        "Command step '{}' not allowed when run_commands=false",
                        step.id
                    ));
                }
                let runner = step
                    .runner
                    .as_deref()
                    .ok_or_else(|| anyhow!("Command step '{}' missing runner", step.id))?;
                if !metadata
                    .permissions
                    .allowed_runners
                    .iter()
                    .any(|r| r == runner)
                {
                    return Err(anyhow!(
                        "Command step '{}' runner '{}' not in allowed_runners",
                        step.id,
                        runner
                    ));
                }
                if step.cmd.as_deref().unwrap_or_default().trim().is_empty() {
                    return Err(anyhow!("Command step '{}' has empty cmd", step.id));
                }
            }
            StepType::Llm => {
                if !metadata.permissions.network_access {
                    let model = step.model.as_deref().unwrap_or_default();
                    if model != "executor" {
                        return Err(anyhow!(
                            "network_access=false requires offline/mock model='executor' for step '{}'",
                            step.id
                        ));
                    }
                }
            }
            StepType::Output => {}
        }
    }

    Ok(())
}
