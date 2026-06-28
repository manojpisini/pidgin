use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ActionRegistry {
    pub safe: Vec<String>,
    pub controlled: Vec<String>,
    pub human_gated: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowEntry {
    pub description: String,
    pub risk_default: String,
    pub allowed_modes: Vec<String>,
    pub required_inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub recommended_executor: String,
    pub fallback_executor: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRegistry {
    pub workflows: BTreeMap<String, WorkflowEntry>,
}

#[derive(Debug, Deserialize)]
pub struct HumanRequired {
    pub actions: Vec<String>,
    pub risk_levels: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockIf {
    pub action_in_do_and_deny: bool,
    pub private_path_referenced: bool,
    pub unknown_workflow: bool,
    pub invalid_mode: bool,
    pub missing_required_field: bool,
    pub dangerous_action_without_human: bool,
}

#[derive(Debug, Deserialize)]
pub struct SafetyRules {
    pub default_deny: Vec<String>,
    pub private_paths: Vec<String>,
    pub human_required: HumanRequired,
    pub block_if: BlockIf,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub fn load_action_registry(path: &Path) -> Result<ActionRegistry, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let registry: ActionRegistry = serde_yaml::from_str(&content)?;
    Ok(registry)
}

pub fn load_workflow_registry(path: &Path) -> Result<WorkflowRegistry, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let registry: WorkflowRegistry = serde_yaml::from_str(&content)?;
    Ok(registry)
}

pub fn load_safety_rules(path: &Path) -> Result<SafetyRules, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let rules: SafetyRules = serde_yaml::from_str(&content)?;
    Ok(rules)
}
