use crate::ast::{FieldValue, PgnPacket};
use crate::registry::{ActionRegistry, SafetyRules, WorkflowRegistry};

#[derive(Debug, Clone, PartialEq)]
pub enum SafetyRuleId {
    Sg1,
    Sg2,
    Sg3,
    Sg4,
    Sg5,
    Sg6,
    Sg7,
    Sg8,
    Sg9,
}

impl std::fmt::Display for SafetyRuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafetyRuleId::Sg1 => write!(f, "SG-1"),
            SafetyRuleId::Sg2 => write!(f, "SG-2"),
            SafetyRuleId::Sg3 => write!(f, "SG-3"),
            SafetyRuleId::Sg4 => write!(f, "SG-4"),
            SafetyRuleId::Sg5 => write!(f, "SG-5"),
            SafetyRuleId::Sg6 => write!(f, "SG-6"),
            SafetyRuleId::Sg7 => write!(f, "SG-7"),
            SafetyRuleId::Sg8 => write!(f, "SG-8"),
            SafetyRuleId::Sg9 => write!(f, "SG-9"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafetyResult {
    pub allowed: bool,
    pub blocked: bool,
    pub fired_rules: Vec<SafetyRuleId>,
    pub human_required: bool,
    pub effective_risk: String,
}

#[derive(Debug, Clone)]
pub struct BlockReason {
    pub rule: SafetyRuleId,
    pub message: String,
}

pub fn check_safety(
    packet: &PgnPacket,
    action_registry: &ActionRegistry,
    safety_rules: &SafetyRules,
    workflows: &WorkflowRegistry,
) -> SafetyResult {
    let mut fired_rules = Vec::new();
    let mut human_required = false;

    // Compute effective risk
    let effective_risk = compute_effective_risk(packet, workflows);

    // SG-1: do and deny conflict
    if let (Some(FieldValue::List(do_list)), Some(FieldValue::List(denied_list))) =
        (packet.fields.get("do"), packet.fields.get("deny"))
    {
        for action in do_list {
            if denied_list.contains(action) {
                fired_rules.push(SafetyRuleId::Sg1);
                break;
            }
        }
    }

    // SG-2: human_gated action without human=yes
    if let Some(FieldValue::List(do_list)) = packet.fields.get("do") {
        let human_field = packet.fields.get("human");
        let has_human_yes = matches!(human_field, Some(FieldValue::Scalar(s)) if s == "yes");

        for action in do_list {
            if action_registry.human_gated.contains(action) && !has_human_yes {
                fired_rules.push(SafetyRuleId::Sg2);
                break;
            }
        }
    }

    // SG-3: high/crit risk defaults to human=yes, explicit human=no on high/crit blocks
    if effective_risk == "high" || effective_risk == "crit" {
        human_required = true;
        if let Some(FieldValue::Scalar(human)) = packet.fields.get("human") {
            if human == "no" {
                fired_rules.push(SafetyRuleId::Sg3);
            }
        }
    }

    // SG-4: private path referenced (check unresolved reference strings for now)
    if let Some(FieldValue::List(references)) = packet.fields.get("in") {
        for reference in references {
            for pattern in &safety_rules.private_paths {
                if reference.contains(pattern) || reference_matches_pattern(reference, pattern) {
                    fired_rules.push(SafetyRuleId::Sg4);
                    break;
                }
            }
        }
    }

    // SG-5: unknown workflow
    if let Some(FieldValue::Scalar(wf_name)) = packet.fields.get("wf") {
        if !workflows.workflows.contains_key(wf_name) {
            fired_rules.push(SafetyRuleId::Sg5);
        }
    }

    // SG-6: invalid mode
    if let (Some(FieldValue::Scalar(wf_name)), Some(FieldValue::Scalar(mode))) =
        (packet.fields.get("wf"), packet.fields.get("mode"))
    {
        if let Some(workflow) = workflows.workflows.get(wf_name) {
            if !workflow.allowed_modes.contains(mode) {
                fired_rules.push(SafetyRuleId::Sg6);
            }
        }
    }

    // SG-7: note field is never parsed for instructions (tested implicitly - no action taken)

    // SG-8: missing required inputs (checked at expansion time, not here)

    // SG-9: critical risk requires approval packet (checked at expansion time)

    let blocked = !fired_rules.is_empty();
    let allowed = !blocked;

    SafetyResult {
        allowed,
        blocked,
        fired_rules,
        human_required,
        effective_risk,
    }
}

fn compute_effective_risk(packet: &PgnPacket, workflows: &WorkflowRegistry) -> String {
    let declared_risk = packet
        .fields
        .get("risk")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("med");

    let workflow_default = packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => workflows.workflows.get(s),
            _ => None,
        })
        .map(|w| w.risk_default.as_str())
        .unwrap_or("med");

    // If the packet declares a risk, use it; otherwise use workflow default
    if packet.fields.contains_key("risk") {
        declared_risk.to_string()
    } else {
        workflow_default.to_string()
    }
}

fn reference_matches_pattern(reference: &str, pattern: &str) -> bool {
    // Simple glob-like matching for private path patterns
    if pattern.starts_with("**/") {
        let suffix = &pattern[3..];
        reference.contains(suffix)
    } else if pattern.ends_with(".*") {
        let prefix = &pattern[..pattern.len() - 2];
        reference.starts_with(prefix)
    } else if pattern.starts_with('.') {
        // Dot-prefixed patterns like .env, .env.*
        reference == pattern || reference.starts_with(&format!("{}.", pattern))
    } else {
        reference == pattern
    }
}
