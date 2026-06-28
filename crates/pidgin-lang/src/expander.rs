use serde::Serialize;

use crate::ast::{FieldValue, PgnPacket};
use crate::registry::WorkflowRegistry;
use crate::resolver::ResolvedRef;
use crate::safety::SafetyResult;

#[derive(Debug, Serialize)]
pub struct ExpandedRef {
    pub reference: String,
    pub status: String,
    pub confidence: f32,
    pub path: Option<String>,
}

impl From<&ResolvedRef> for ExpandedRef {
    fn from(r: &ResolvedRef) -> Self {
        ExpandedRef {
            reference: r.original.clone(),
            status: format!("{:?}", r.status),
            confidence: r.confidence,
            path: r.resolved_path.as_ref().map(|p| p.display().to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExpandedRunPacket {
    pub spec_version: String,
    pub run_id: String,
    pub workflow: String,
    pub mode: String,
    pub inputs: Vec<ExpandedRef>,
    pub outputs: Vec<ExpandedRef>,
    pub do_actions: Vec<String>,
    pub deny_actions: Vec<String>,
    pub effective_risk: String,
    pub human_required: bool,
    pub recommended_executor: String,
    pub fallback_executor: String,
    pub ttl: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExpandedApprovalPacket {
    pub spec_version: String,
    pub run_id: String,
    pub workflow: String,
    pub risk: String,
    pub human_required: bool,
    pub actions: Vec<String>,
}

pub fn expand_to_run_packet(
    packet: &PgnPacket,
    resolved_refs: &[ResolvedRef],
    safety: &SafetyResult,
    workflows: &WorkflowRegistry,
) -> ExpandedRunPacket {
    let wf_name = packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("unknown");

    let workflow_entry = workflows.workflows.get(wf_name);

    let inputs: Vec<ExpandedRef> = resolved_refs
        .iter()
        .filter(|r| matches!(packet.fields.get("in"), Some(FieldValue::List(refs)) if refs.contains(&r.original)))
        .map(ExpandedRef::from)
        .collect();

    let outputs: Vec<ExpandedRef> = resolved_refs
        .iter()
        .filter(|r| matches!(packet.fields.get("out"), Some(FieldValue::List(refs)) if refs.contains(&r.original)))
        .map(ExpandedRef::from)
        .collect();

    let do_actions = packet
        .fields
        .get("do")
        .and_then(|v| match v {
            FieldValue::List(items) => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let deny_actions = packet
        .fields
        .get("deny")
        .and_then(|v| match v {
            FieldValue::List(items) => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let note = packet
        .fields
        .get("note")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => Some(s.clone()),
            _ => None,
        });

    let recommended_executor = workflow_entry
        .map(|w| w.recommended_executor.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let fallback_executor = workflow_entry
        .map(|w| w.fallback_executor.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let ttl = packet
        .fields
        .get("ttl")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "24h".to_string());

    ExpandedRunPacket {
        spec_version: "1.0".to_string(),
        run_id: packet.run_id.clone(),
        workflow: wf_name.to_string(),
        mode: packet
            .fields
            .get("mode")
            .and_then(|v| match v {
                FieldValue::Scalar(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "unknown".to_string()),
        inputs,
        outputs,
        do_actions,
        deny_actions,
        effective_risk: safety.effective_risk.clone(),
        human_required: safety.human_required,
        recommended_executor,
        fallback_executor,
        ttl,
        note,
    }
}

pub fn expand_to_approval_request(
    packet: &PgnPacket,
    safety: &SafetyResult,
    _workflows: &WorkflowRegistry,
) -> ExpandedApprovalPacket {
    let wf_name = packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("unknown");

    let actions = packet
        .fields
        .get("do")
        .and_then(|v| match v {
            FieldValue::List(items) => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default();

    ExpandedApprovalPacket {
        spec_version: "1.0".to_string(),
        run_id: packet.run_id.clone(),
        workflow: wf_name.to_string(),
        risk: safety.effective_risk.clone(),
        human_required: safety.human_required,
        actions,
    }
}
