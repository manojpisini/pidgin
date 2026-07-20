use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::ast::{FieldValue, PgnPacket};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolutionStatus {
    Resolved,
    Missing,
    Unresolved,
    Forbidden,
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
        "file" => resolve_path_ref(&original, &ref_id, &ctx.host_root, required, "file", |p| p.exists()),
        "folder" => resolve_path_ref(&original, &ref_id, &ctx.host_root, required, "folder", |p| p.is_dir()),
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

fn resolve_symlink_target(path: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 || !path.is_symlink() {
        return None;
    }
    let target = std::fs::read_link(path).ok()?;
    let absolute = if target.is_absolute() {
        target
    } else if let Some(parent) = path.parent() {
        parent.join(&target)
    } else {
        target
    };
    if absolute.is_symlink() {
        resolve_symlink_target(&absolute, depth - 1)
    } else {
        Some(absolute)
    }
}

fn is_path_within_root(candidate: &Path, root: &Path) -> bool {
    let canonical_root = match root.canonicalize() {
        Ok(r) => r,
        Err(_) => return false,
    };

    // Resolve symlinks explicitly if present
    if let Some(sym_target) = resolve_symlink_target(candidate, 8)
        && !sym_target.starts_with(&canonical_root)
    {
        return false;
    }

    // If the path exists, use canonicalize for proper symlink resolution
    if let Ok(canonical) = candidate.canonicalize() {
        return canonical.starts_with(&canonical_root);
    }

    // For non-existent paths: verify component-by-component that the path
    // doesn't use ParentDir to escape above canonical_root
    let root_comps: Vec<_> = canonical_root.components().collect();
    let candidate_comps: Vec<_> = candidate.components().collect();

    if candidate_comps.len() < root_comps.len() {
        return false;
    }

    // First root_len components must match root exactly
    for (i, rc) in root_comps.iter().enumerate() {
        if candidate_comps.get(i) != Some(rc) {
            return false;
        }
    }

    // Remaining components must not escape above root via ParentDir
    let mut relative_depth: isize = 0;
    for comp in &candidate_comps[root_comps.len()..] {
        match comp {
            std::path::Component::ParentDir => {
                relative_depth -= 1;
                if relative_depth < 0 {
                    return false;
                }
            }
            std::path::Component::Normal(_) | std::path::Component::CurDir => {
                relative_depth += 1;
            }
            _ => return false,
        }
    }

    true
}

fn resolve_path_ref(
    original: &str,
    ref_id: &str,
    host_root: &Path,
    required: bool,
    namespace: &str,
    exists_fn: fn(&Path) -> bool,
) -> ResolvedRef {
    let canonical_root = match host_root.canonicalize() {
        Ok(r) => r,
        Err(_) => {
            return ResolvedRef {
                original: original.to_string(),
                namespace: namespace.to_string(),
                ref_id: ref_id.to_string(),
                resolved_path: None,
                confidence: 0.0,
                required,
                status: ResolutionStatus::Forbidden,
            };
        }
    };
    let resolved_path = canonical_root.join(ref_id);

    if !is_path_within_root(&resolved_path, &canonical_root) {
        return ResolvedRef {
            original: original.to_string(),
            namespace: namespace.to_string(),
            ref_id: ref_id.to_string(),
            resolved_path: None,
            confidence: 0.0,
            required,
            status: ResolutionStatus::Forbidden,
        };
    }

    let (status, confidence) = if exists_fn(&resolved_path) {
        (ResolutionStatus::Resolved, 1.0)
    } else {
        (ResolutionStatus::Missing, 0.0)
    };

    ResolvedRef {
        original: original.to_string(),
        namespace: namespace.to_string(),
        ref_id: ref_id.to_string(),
        resolved_path: Some(resolved_path),
        confidence,
        required,
        status,
    }
}

pub fn load_aliases(path: &Path) -> Result<ReferenceAliases, crate::registry::ConfigError> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > 10_485_760 {
        return Err(crate::registry::ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "config file {} exceeds maximum size ({} > 10 MiB)",
                path.display(),
                metadata.len()
            ),
        )));
    }
    let content = std::fs::read_to_string(path)?;
    let aliases: ReferenceAliases = serde_yaml::from_str(&content)?;
    Ok(aliases)
}
