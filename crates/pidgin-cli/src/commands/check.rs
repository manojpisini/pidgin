use std::fs;
use std::path::PathBuf;

use pidgin_lang::parser::parse_packet;
use pidgin_lang::resolver::{ResolutionStatus, ResolverContext, resolve_all};
use pidgin_lang::safety::{check_resolved_refs_safety, check_safety};
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

use super::{canonicalize_host, get_required_inputs, load_pipeline_configs};

pub fn run(file: PathBuf, host: PathBuf, json: bool) {
    let cfg = load_pipeline_configs(&host);
    let host_root = canonicalize_host(&host);
    let content = match fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: FAIL (read error: {})", file.display(), e);
            std::process::exit(1);
        }
    };
    let packet = match parse_packet(&content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: FAIL (parse error: {})", file.display(), e);
            std::process::exit(1);
        }
    };

    let syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &cfg.workflows);
    let mut all_errors: Vec<String> = Vec::new();
    for err in &syntax_errors {
        all_errors.push(format!("[{}] {}", err.code, err.message));
    }
    for err in &schema_errors {
        all_errors.push(format!("[{}] {}", err.code, err.message));
    }

    let safety_result = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
    for rule in &safety_result.fired_rules {
        all_errors.push(format!("[{}] (safety)", rule));
    }

    let required_inputs = get_required_inputs(&packet, &cfg.workflows);

    let ctx = ResolverContext {
        host_root,
        aliases: cfg.aliases,
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);
    let resolved_fired = check_resolved_refs_safety(&resolved, &cfg.safety_rules.private_paths);
    for rule in &resolved_fired {
        all_errors.push(format!("[{}] (safety after resolution)", rule));
    }
    for r in &resolved {
        if r.required
            && matches!(
                r.status,
                ResolutionStatus::Unresolved | ResolutionStatus::Forbidden
            )
        {
            let label = match r.status {
                ResolutionStatus::Unresolved => "UNRESOLVED",
                ResolutionStatus::Forbidden => "FORBIDDEN",
                _ => "ERROR",
            };
            all_errors.push(format!("[{}] {} (required)", label, r.original));
        }
    }

    if json {
        let output = serde_json::json!({
            "file": file.display().to_string(),
            "valid": all_errors.is_empty(),
            "errors": all_errors,
            "safety": {
                "allowed": safety_result.allowed,
                "blocked": safety_result.blocked,
                "fired_rules": safety_result.fired_rules.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
                "human_required": safety_result.human_required,
                "effective_risk": safety_result.effective_risk,
            },
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        if all_errors.is_empty() {
            println!("{}: PASS", file.display());
        } else {
            println!("{}: FAIL", file.display());
            for err in &all_errors {
                eprintln!(" {}", err);
            }
        }
    }

    if !all_errors.is_empty() {
        let has_validation = !syntax_errors.is_empty() || !schema_errors.is_empty();
        std::process::exit(if has_validation { 1 } else { 2 });
    }
}
