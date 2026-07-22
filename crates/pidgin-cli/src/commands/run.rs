use std::fs;
use std::path::PathBuf;

use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::logging::{log_event, LogEvent};
use pidgin_lang::parser::parse_packet;
use pidgin_lang::resolver::{resolve_all, ResolutionStatus, ResolverContext};
use pidgin_lang::router::{explain_route, route};
use pidgin_lang::safety::{check_resolved_refs_safety, check_safety};
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

use super::{canonicalize_host, get_required_inputs, load_pipeline_configs};

pub fn run(file: PathBuf, host: PathBuf, out: Option<PathBuf>) {
    let cfg = load_pipeline_configs(&host);
    let host_root = canonicalize_host(&host);
    let run_id;

    let content = fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("{}: Error reading file: {}", file.display(), e);
        std::process::exit(1);
    });
    let packet = match parse_packet(&content) {
        Ok(p) => {
            run_id = p.run_id.clone();
            p
        }
        Err(e) => {
            eprintln!("{}: Parse error: {}", file.display(), e);
            let _ = log_event(
                &host
                    .join(".pidgin")
                    .join("logs")
                    .join("PIDGIN_RUNTIME_RUNS.csv"),
                &LogEvent::Parse {
                    run_id: file.display().to_string(),
                    ok: false,
                },
            );
            std::process::exit(1);
        }
    };
    let _ = log_event(
        &host
            .join(".pidgin")
            .join("logs")
            .join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::Parse {
            run_id: run_id.clone(),
            ok: true,
        },
    );

    let syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &cfg.workflows);
    if !syntax_errors.is_empty() || !schema_errors.is_empty() {
        eprintln!("{}: Validation errors", file.display());
        for err in &syntax_errors {
            eprintln!("  [{}] {}", err.code, err.message);
        }
        for err in &schema_errors {
            eprintln!("  [{}] {}", err.code, err.message);
        }
        let _ = log_event(
            &host
                .join(".pidgin")
                .join("logs")
                .join("PIDGIN_RUNTIME_RUNS.csv"),
            &LogEvent::Validate {
                run_id: run_id.clone(),
                ok: false,
            },
        );
        std::process::exit(1);
    }
    let _ = log_event(
        &host
            .join(".pidgin")
            .join("logs")
            .join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::Validate {
            run_id: run_id.clone(),
            ok: true,
        },
    );

    let safety = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
    let rules_str = safety
        .fired_rules
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let _ = log_event(
        &host
            .join(".pidgin")
            .join("logs")
            .join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::SafetyGate {
            run_id: run_id.clone(),
            blocked: safety.blocked,
            rules: rules_str,
        },
    );
    if safety.blocked {
        eprintln!("{}: Blocked by safety", file.display());
        for rule in &safety.fired_rules {
            eprintln!("  [{}]", rule);
        }
        std::process::exit(2);
    }

    let required_inputs = get_required_inputs(&packet, &cfg.workflows);
    let ctx = ResolverContext {
        host_root,
        aliases: cfg.aliases,
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);

    let resolved_fired = check_resolved_refs_safety(&resolved, &cfg.safety_rules.private_paths);
    if !resolved_fired.is_empty() {
        for rule in &resolved_fired {
            eprintln!(
                "{}: Blocked by safety after resolution: {}",
                file.display(),
                rule
            );
        }
        std::process::exit(2);
    }

    let unresolved = resolved
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                ResolutionStatus::Unresolved | ResolutionStatus::Forbidden
            )
        })
        .count();
    let _ = log_event(
        &host
            .join(".pidgin")
            .join("logs")
            .join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::Resolve {
            run_id: run_id.clone(),
            refs_total: resolved.len(),
            refs_unresolved: unresolved,
        },
    );
    for r in &resolved {
        if r.required
            && matches!(
                r.status,
                ResolutionStatus::Unresolved | ResolutionStatus::Forbidden
            )
        {
            eprintln!(
                "{}: Required reference {}: {}",
                file.display(),
                match r.status {
                    ResolutionStatus::Forbidden => "forbidden (path traversal blocked)",
                    _ => "unresolved",
                },
                r.original
            );
            std::process::exit(3);
        }
    }

    let decision = route(&packet, &cfg.workflows, &safety);
    let expanded = expand_to_run_packet(&packet, &resolved, &safety, &cfg.workflows);
    let yaml = serde_yaml::to_string(&expanded).unwrap_or_else(|e| {
        eprintln!("Error serializing: {}", e);
        std::process::exit(5);
    });
    let _ = log_event(
        &host
            .join(".pidgin")
            .join("logs")
            .join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::Expand {
            run_id: run_id.clone(),
            packet_type: "run".to_string(),
        },
    );

    match out {
        Some(path) => {
            fs::write(&path, &yaml).unwrap_or_else(|e| {
                eprintln!("Error writing output: {}", e);
                std::process::exit(5);
            });
            println!(
                "{}: expanded -> {} (dry-run)",
                file.display(),
                path.display()
            );
        }
        None => {
            println!("---");
            println!("{}", yaml.trim());
            println!("---");
            println!("Route: {}", explain_route(&decision));
        }
    }

    let _ = log_event(
        &host
            .join(".pidgin")
            .join("logs")
            .join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::Run {
            run_id: run_id.clone(),
            status: "dry_run_ok".to_string(),
        },
    );
}
