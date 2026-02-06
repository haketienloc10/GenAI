use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub metadata: SkillMetadata,
    pub markdown_body: String,
    pub steps: Vec<WorkflowStep>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: String,
    pub tags: Vec<String>,

    pub entrypoint: String,
    pub workflow_version: u32,

    pub capabilities: Capabilities,
    pub permissions: Permissions,
    pub response_format: ResponseFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub requires_repo: bool,
    pub supports_interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub run_commands: bool,
    pub allowed_runners: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub network_access: bool,
    pub write_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    Command,
    Llm,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    #[serde(rename = "type")]
    pub step_type: StepType,
    #[serde(rename = "if")]
    pub if_expr: Option<String>,
    pub output_var: Option<String>,

    pub runner: Option<String>,
    pub cmd: Option<String>,

    pub model: Option<String>,
    #[serde(default)]
    pub input_vars: Vec<String>,
    pub prompt: Option<String>,

    pub format: Option<String>,
    pub template: Option<String>,
}
