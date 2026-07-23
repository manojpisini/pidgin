use serde::Serialize;

use crate::ast::{FieldValue, PgnPacket};
use crate::registry::WorkflowRegistry;
use crate::safety::SafetyResult;

#[derive(Debug, Serialize)]
pub struct RouteDecision {
    pub recommended_executor: String,
    pub fallback_executor: String,
    pub reason: String,
    pub human_required: bool,
}

pub fn route(
    packet: &PgnPacket,
    registry: &WorkflowRegistry,
    safety: &SafetyResult,
) -> RouteDecision {
    let wf_name = packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("unknown");

    let explicit_route = packet.fields.get("route").and_then(|v| match v {
        FieldValue::Scalar(s) => Some(s.clone()),
        _ => None,
    });

    let (recommended, fallback) = if let Some(route_val) = &explicit_route {
        (
            route_val.clone(),
            registry
                .workflows
                .get(wf_name)
                .map(|w| w.fallback_executor.clone())
                .unwrap_or_else(|| "unknown".to_string()),
        )
    } else if let Some(entry) = registry.workflows.get(wf_name) {
        (
            entry.recommended_executor.clone(),
            entry.fallback_executor.clone(),
        )
    } else {
        ("unknown".to_string(), "unknown".to_string())
    };

    RouteDecision {
        recommended_executor: recommended,
        fallback_executor: fallback,
        reason: if explicit_route.is_some() {
            "explicit_route".to_string()
        } else {
            format!("workflow_{}", wf_name)
        },
        human_required: safety.human_required,
    }
}

pub fn explain_route(decision: &RouteDecision) -> String {
    format!(
        "Route: {} (fallback: {}), human_required: {}, reason: {}",
        decision.recommended_executor,
        decision.fallback_executor,
        decision.human_required,
        decision.reason
    )
}
