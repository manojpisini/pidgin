use crate::ast::{Directive, FieldValue, PgnPacket};
use super::ValidationError;

pub fn validate_syntax(packet: &PgnPacket) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    match packet.directive {
        Directive::Run => {
            require_field(packet, "wf", &mut errors);
            require_field(packet, "mode", &mut errors);
            require_list_field(packet, "in", &mut errors);
            require_list_field(packet, "out", &mut errors);
        }
        Directive::Result => {
            require_field(packet, "wf", &mut errors);
            require_list_field(packet, "out", &mut errors);
            require_field(packet, "status", &mut errors);
        }
        Directive::Approval => {
            require_field(packet, "status", &mut errors);
        }
        Directive::Context => {
            require_field(packet, "wf", &mut errors);
            require_list_field(packet, "in", &mut errors);
        }
    }

    errors
}

fn require_field(packet: &PgnPacket, field: &str, errors: &mut Vec<ValidationError>) {
    if !packet.fields.contains_key(field) {
        errors.push(ValidationError {
            code: "PGN_E001".to_string(),
            message: format!("missing required field: {}", field),
        });
    }
}

fn require_list_field(packet: &PgnPacket, field: &str, errors: &mut Vec<ValidationError>) {
    match packet.fields.get(field) {
        None => {
            errors.push(ValidationError {
                code: "PGN_E001".to_string(),
                message: format!("missing required field: {}", field),
            });
        }
        Some(value) => {
            if !matches!(value, FieldValue::List(_)) {
                errors.push(ValidationError {
                    code: "PGN_E016".to_string(),
                    message: format!("field '{}' must be a list, got scalar", field),
                });
            }
        }
    }
}
