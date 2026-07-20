use serde::Serialize;
use std::path::Path;

use crate::ast::PgnPacket;
use crate::resolver::{ResolutionStatus, ResolvedRef};

#[derive(Debug, Serialize)]
pub struct ContextPlan {
    pub run_id: String,
    pub primary_refs: Vec<ContextRef>,
    pub retrieval_methods: Vec<String>,
    pub token_budget: usize,
    pub compression_allowed: bool,
    pub total_bytes: usize,
    pub estimated_tokens: usize,
}

#[derive(Debug, Serialize)]
pub struct ContextRef {
    pub reference: String,
    pub resolved: bool,
    pub path: Option<String>,
    pub bytes: Option<usize>,
    pub retrieval_method: String,
}

fn pick_retrieval_method(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h") => {
            "full_text".to_string()
        }
        Some("yaml" | "yml" | "json" | "toml") => "full_text".to_string(),
        Some("md" | "txt" | "rst") => "full_text".to_string(),
        Some("png" | "jpg" | "jpeg" | "gif" | "svg") => "metadata_only".to_string(),
        Some("pdf") => "metadata_only".to_string(),
        _ => "full_text".to_string(),
    }
}

pub fn build_context_plan(packet: &PgnPacket, resolved: &[ResolvedRef]) -> ContextPlan {
    let mut total_bytes = 0usize;
    let primary_refs: Vec<ContextRef> = resolved
        .iter()
        .map(|r| {
            let (bytes, method) = match (&r.resolved_path, r.status) {
                (Some(path), ResolutionStatus::Resolved) => {
                    let size = std::fs::metadata(path)
                        .map(|m| m.len() as usize)
                        .unwrap_or(0);
                    let method = pick_retrieval_method(path);
                    (Some(size), method)
                }
                _ => (None, "full_text".to_string()),
            };
            if let Some(b) = bytes {
                total_bytes += b;
            }
            ContextRef {
                reference: r.original.clone(),
                resolved: matches!(r.status, ResolutionStatus::Resolved),
                path: r.resolved_path.as_ref().map(|p| p.display().to_string()),
                bytes,
                retrieval_method: method,
            }
        })
        .collect();

    let estimated_tokens = total_bytes.div_ceil(4);
    let token_budget = estimated_tokens.max(1000);

    // Pick retrieval methods used across all refs
    let mut methods: Vec<String> = primary_refs
        .iter()
        .map(|r| r.retrieval_method.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    methods.sort();

    ContextPlan {
        run_id: packet.run_id.clone(),
        primary_refs,
        retrieval_methods: methods,
        token_budget,
        compression_allowed: total_bytes > 50_000,
        total_bytes,
        estimated_tokens,
    }
}
