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

    for step in &skill.steps {
        if !ids.insert(step.id.clone()) {
            return Err(anyhow!("Duplicate step id: {}", step.id));
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

#[cfg(test)]
mod tests {
    use super::validate_skill;
    use crate::skill::model::{
        Capabilities, Permissions, ResponseFormat, Skill, SkillMetadata, StepType, WorkflowStep,
    };

    #[test]
    fn allows_reusing_output_var_for_fallback_steps() {
        let skill = base_skill(vec![
            WorkflowStep {
                id: "get_staged_diff".to_string(),
                step_type: StepType::Command,
                if_expr: None,
                output_var: Some("diff".to_string()),
                runner: Some("bash".to_string()),
                cmd: Some("git diff --staged".to_string()),
                model: None,
                input_vars: vec![],
                prompt: None,
                format: None,
                template: None,
            },
            WorkflowStep {
                id: "fallback_unstaged".to_string(),
                step_type: StepType::Command,
                if_expr: Some("{{diff}} == ''".to_string()),
                output_var: Some("diff".to_string()),
                runner: Some("bash".to_string()),
                cmd: Some("git diff".to_string()),
                model: None,
                input_vars: vec![],
                prompt: None,
                format: None,
                template: None,
            },
        ]);

        let result = validate_skill(&skill);
        assert!(
            result.is_ok(),
            "expected validation success, got {result:?}"
        );
    }

    #[test]
    fn still_rejects_duplicate_step_ids() {
        let skill = base_skill(vec![
            WorkflowStep {
                id: "duplicate".to_string(),
                step_type: StepType::Output,
                if_expr: None,
                output_var: None,
                runner: None,
                cmd: None,
                model: None,
                input_vars: vec![],
                prompt: None,
                format: Some("text".to_string()),
                template: Some("one".to_string()),
            },
            WorkflowStep {
                id: "duplicate".to_string(),
                step_type: StepType::Output,
                if_expr: None,
                output_var: None,
                runner: None,
                cmd: None,
                model: None,
                input_vars: vec![],
                prompt: None,
                format: Some("text".to_string()),
                template: Some("two".to_string()),
            },
        ]);

        let result = validate_skill(&skill);
        assert!(result.is_err(), "expected validation error");
    }

    fn base_skill(steps: Vec<WorkflowStep>) -> Skill {
        Skill {
            metadata: SkillMetadata {
                name: "auto-commit-msg".to_string(),
                description: "desc".to_string(),
                version: "1.0.0".to_string(),
                category: "git".to_string(),
                tags: vec![],
                entrypoint: "workflow".to_string(),
                workflow_version: 1,
                capabilities: Capabilities {
                    requires_repo: true,
                    supports_interactive: false,
                },
                permissions: Permissions {
                    run_commands: true,
                    allowed_runners: vec!["bash".to_string()],
                    allowed_paths: vec![],
                    network_access: true,
                    write_access: false,
                },
                response_format: ResponseFormat {
                    format_type: "text".to_string(),
                    style: None,
                },
            },
            markdown_body: String::new(),
            steps,
            path: "skills/auto-commit-msg/SKILL.md".to_string(),
        }
    }
}
