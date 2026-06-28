use crate::ast::{FieldValue, PgnPacket};
use crate::registry::WorkflowRegistry;
use super::ValidationError;

pub fn validate_schema(packet: &PgnPacket, workflows: &WorkflowRegistry) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check workflow exists
    if let Some(FieldValue::Scalar(wf_name)) = packet.fields.get("wf") {
        if !workflows.workflows.contains_key(wf_name) {
            errors.push(ValidationError {
                code: "PGN_E002".to_string(),
                message: format!("unknown workflow: {}", wf_name),
            });
            return errors;
        }

        let workflow = &workflows.workflows[wf_name];

        // Check mode is allowed
        if let Some(FieldValue::Scalar(mode)) = packet.fields.get("mode")
            && !workflow.allowed_modes.contains(mode)
        {
            errors.push(ValidationError {
                code: "PGN_E003".to_string(),
                message: format!(
                    "invalid mode '{}' for workflow '{}'; allowed: {:?}",
                    mode, wf_name, workflow.allowed_modes
                ),
            });
        }

        // Check risk level is valid
        if let Some(FieldValue::Scalar(risk)) = packet.fields.get("risk")
            && !["low", "med", "high", "crit"].contains(&risk.as_str())
        {
            errors.push(ValidationError {
                code: "PGN_E004".to_string(),
                message: format!("invalid risk level: {}", risk),
            });
        }
    }

    errors
}
