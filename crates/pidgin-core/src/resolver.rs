use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::ast::{FieldValue, PgnPacket};

#[derive(Debug, Clone, PartialEq)]
pub enum ResolutionStatus {
    Resolved,
    Missing,
    Unresolved,
}

#[derive(Debug, Clone)]
pub struct ResolvedRef {
    pub original: String,
    pub namespace: String,
    pub ref_id: String,
    pub resolved_path: Option<PathBuf>,
    pub confidence: f32,
    pub required: bool,
    pub status: ResolutionStatus,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceAliases {
    pub aliases: BTreeMap<String, String>,
    pub common: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct ResolverContext {
    pub host_root: PathBuf,
    pub aliases: ReferenceAliases,
    pub required_inputs: Vec<String>,
}

pub fn parse_ref(reference: &str) -> Option<(String, String)> {
    let trimmed = reference.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(idx) = trimmed.find(':') {
        let namespace = trimmed[..idx].trim().to_string();
        let ref_id = trimmed[idx + 1..].trim().to_string();
        if !namespace.is_empty() && !ref_id.is_empty() {
            return Some((namespace, ref_id));
        }
    }
    // Bare alias — use empty namespace to signal alias lookup
    Some((String::new(), trimmed.to_string()))
}

pub fn expand_alias(bare: &str, aliases: &ReferenceAliases) -> Option<String> {
    aliases
        .aliases
        .get(bare)
        .or_else(|| aliases.common.get(bare))
        .cloned()
}

pub fn resolve_ref(reference: &str, ctx: &ResolverContext) -> ResolvedRef {
    let original = reference.to_string();
    let required = ctx.required_inputs.contains(&original);

    let parsed = parse_ref(reference);
    let parsed = match parsed {
        Some(p) => p,
        None => {
            return ResolvedRef {
                original,
                namespace: String::new(),
                ref_id: String::new(),
                resolved_path: None,
                confidence: 0.0,
                required,
                status: ResolutionStatus::Unresolved,
            };
        }
    };

    let (namespace, ref_id) = parsed;

    // Bare alias — try to expand
    let (namespace, ref_id) = if namespace.is_empty() {
        match expand_alias(&ref_id, &ctx.aliases) {
            Some(expanded) => {
                match parse_ref(&expanded) {
                    Some((ns, id)) => (ns, id),
                    None => {
                        return ResolvedRef {
                            original,
                            namespace: String::new(),
                            ref_id: ref_id.clone(),
                            resolved_path: None,
                            confidence: 0.3,
                            required,
                            status: ResolutionStatus::Unresolved,
                        };
                    }
                }
            }
            None => {
                return ResolvedRef {
                    original,
                    namespace: String::new(),
                    ref_id: ref_id.clone(),
                    resolved_path: None,
                    confidence: 0.0,
                    required,
                    status: ResolutionStatus::Unresolved,
                };
            }
        }
    } else {
        (namespace, ref_id)
    };

    match namespace.as_str() {
        "file" => resolve_file_ref(&original, &ref_id, &ctx.host_root, required),
        "folder" => resolve_folder_ref(&original, &ref_id, &ctx.host_root, required),
        _ => ResolvedRef {
            original,
            namespace,
            ref_id,
            resolved_path: None,
            confidence: 0.0,
            required,
            status: ResolutionStatus::Unresolved,
        },
    }
}

pub fn resolve_all(packet: &PgnPacket, ctx: &ResolverContext) -> Vec<ResolvedRef> {
    let mut results = Vec::new();

    for field_name in &["in", "out"] {
        if let Some(FieldValue::List(refs)) = packet.fields.get(*field_name) {
            for reference in refs {
                results.push(resolve_ref(reference, ctx));
            }
        }
    }

    results
}

fn resolve_file_ref(
    original: &str,
    ref_id: &str,
    host_root: &Path,
    required: bool,
) -> ResolvedRef {
    let resolved_path = host_root.join(ref_id);

    let (status, confidence) = if resolved_path.exists() {
        (ResolutionStatus::Resolved, 1.0)
    } else {
        (ResolutionStatus::Missing, 0.0)
    };

    ResolvedRef {
        original: original.to_string(),
        namespace: "file".to_string(),
        ref_id: ref_id.to_string(),
        resolved_path: Some(resolved_path),
        confidence,
        required,
        status,
    }
}

fn resolve_folder_ref(
    original: &str,
    ref_id: &str,
    host_root: &Path,
    required: bool,
) -> ResolvedRef {
    let resolved_path = host_root.join(ref_id);

    let (status, confidence) = if resolved_path.is_dir() {
        (ResolutionStatus::Resolved, 1.0)
    } else {
        (ResolutionStatus::Missing, 0.0)
    };

    ResolvedRef {
        original: original.to_string(),
        namespace: "folder".to_string(),
        ref_id: ref_id.to_string(),
        resolved_path: Some(resolved_path),
        confidence,
        required,
        status,
    }
}

pub fn load_aliases(path: &Path) -> Result<ReferenceAliases, crate::registry::ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let aliases: ReferenceAliases = serde_yaml::from_str(&content)?;
    Ok(aliases)
}
