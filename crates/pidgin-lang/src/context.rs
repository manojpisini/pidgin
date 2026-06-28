use serde::Serialize;

use crate::ast::PgnPacket;
use crate::resolver::{ResolvedRef, ResolutionStatus};

#[derive(Debug, Serialize)]
pub struct ContextPlan {
    pub run_id: String,
    pub primary_refs: Vec<ContextRef>,
    pub retrieval_methods: Vec<String>,
    pub token_budget: usize,
    pub compression_allowed: bool,
}

#[derive(Debug, Serialize)]
pub struct ContextRef {
    pub reference: String,
    pub resolved: bool,
    pub path: Option<String>,
}

pub fn build_context_plan(packet: &PgnPacket, resolved: &[ResolvedRef]) -> ContextPlan {
    let primary_refs: Vec<ContextRef> = resolved
        .iter()
        .map(|r| ContextRef {
            reference: r.original.clone(),
            resolved: matches!(r.status, ResolutionStatus::Resolved),
            path: r
                .resolved_path
                .as_ref()
                .map(|p| p.display().to_string()),
        })
        .collect();

    let total_chars: usize = resolved
        .iter()
        .map(|r| r.original.len())
        .sum();
    let token_budget = total_chars.div_ceil(4);

    ContextPlan {
        run_id: packet.run_id.clone(),
        primary_refs,
        retrieval_methods: vec![
            "direct_path".to_string(),
            "full_text".to_string(),
        ],
        token_budget: token_budget.max(1000),
        compression_allowed: true,
    }
}
