use std::fs;
use std::path::PathBuf;

use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::logging::{LogEvent, log_event};
use pidgin_lang::parser::parse_packet;
use pidgin_lang::router::{explain_route, route};
use pidgin_lang::safety::check_safety;
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

use super::load_pipeline_configs;

pub fn run(file: PathBuf, host: PathBuf, r#out: Option<PathBuf>, json: bool) {
    let cfg = load_pipeline_configs(&host);
    let content = fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("{}: Error reading file: {}", file.display(), e);
        std::process::exit(1);
    });
    let packet = parse_packet(&content).unwrap_or_else(|e| {
        eprintln!("{}: Parse error: {}", file.display(), e);
        std::process::exit(1);
    });

    let syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &cfg.workflows);
    if !syntax_errors.is_empty() || !schema_errors.is_empty() {
        eprintln!("{}: Cannot expand — validation errors", file.display());
        for err in &syntax_errors {
            eprintln!("  [{}] {}", err.code, err.message);
        }
        for err in &schema_errors {
            eprintln!("  [{}] {}", err.code, err.message);
        }
        std::process::exit(1);
    }

    let safety = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
    let decision = route(&packet, &cfg.workflows, &safety);
    let expanded = expand_to_run_packet(&packet, &[], &safety, &cfg.workflows);

    if json {
        let output = serde_json::json!({
            "packet": &expanded,
            "route": &decision,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        let yaml = serde_yaml::to_string(&expanded).unwrap_or_else(|e| {
            eprintln!("Error serializing expanded packet: {}", e);
            std::process::exit(5);
        });
        match r#out {
            Some(path) => {
                fs::write(&path, &yaml).unwrap_or_else(|e| {
                    eprintln!("Error writing output: {}", e);
                    std::process::exit(5);
                });
                println!("{}: expanded -> {}", file.display(), path.display());
            }
            None => {
                println!("---");
                println!("{}", yaml.trim());
                println!("---");
                println!("Route: {}", explain_route(&decision));
            }
        }
    }

    let _ = log_event(
        &host
            .join(".pidgin")
            .join("logs")
            .join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::Expand {
            run_id: packet.run_id.clone(),
            packet_type: "run".to_string(),
        },
    );
}
