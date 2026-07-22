use std::fs;
use std::path::PathBuf;

use pidgin_lang::parser::parse_packet;
use pidgin_lang::resolver::{resolve_all, ResolutionStatus, ResolverContext};

use super::{canonicalize_host, get_required_inputs, load_pipeline_configs};

pub fn run(file: PathBuf, host: PathBuf, json: bool) {
    let cfg = load_pipeline_configs(&host);
    let host_root = canonicalize_host(&host);
    let content = match fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: Error reading file: {}", file.display(), e);
            std::process::exit(1);
        }
    };
    let packet = match parse_packet(&content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: Parse error: {}", file.display(), e);
            std::process::exit(1);
        }
    };
    let required_inputs = get_required_inputs(&packet, &cfg.workflows);
    let ctx = ResolverContext {
        host_root,
        aliases: cfg.aliases,
        required_inputs,
    };
    let results = resolve_all(&packet, &ctx);
    if json {
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
    } else {
        if results.is_empty() {
            println!("{}: no references found", file.display());
            return;
        }
        println!("{}: {}", file.display(), results.len());
        for r in &results {
            let status = match r.status {
                ResolutionStatus::Resolved => "RESOLVED",
                ResolutionStatus::Missing => "MISSING",
                ResolutionStatus::Unresolved => "UNRESOLVED",
                ResolutionStatus::Forbidden => "FORBIDDEN",
            };
            let path = r
                .resolved_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".to_string());
            println!(
                "  {}  ns={}  id={}  confidence={:.1}  required={}  path={}",
                status, r.namespace, r.ref_id, r.confidence, r.required, path
            );
        }
    }
    let all_ok = results.iter().all(|r| {
        !r.required
            || !matches!(
                r.status,
                ResolutionStatus::Unresolved | ResolutionStatus::Forbidden
            )
    });
    if !all_ok {
        std::process::exit(3);
    }
}
