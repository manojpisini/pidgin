use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use pidgin_core::parser::parse_packet;
use pidgin_core::registry::{load_action_registry, load_safety_rules, load_workflow_registry};
use pidgin_core::resolver::{load_aliases, resolve_all, ResolverContext};
use pidgin_core::safety::check_safety;
use pidgin_core::validator::syntax::validate_syntax;
use pidgin_core::validator::schema::validate_schema;

#[derive(Parser)]
#[command(name = "pgn", about = "Pidgin — A compact agent handoff protocol runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a Pidgin packet and print the AST
    Parse {
        /// Path to the .pgn file
        file: PathBuf,
    },

    /// Validate a Pidgin packet (syntax + schema)
    Validate {
        /// Path to the .pgn file(s)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Host root directory (for config lookup)
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Validate → safety gate, end to end
    Check {
        /// Path to the .pgn file
        file: PathBuf,

        /// Host root directory (for config lookup)
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Resolve all short references in a packet
    Resolve {
        /// Path to the .pgn file
        file: PathBuf,

        /// Host root directory (for config lookup)
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { file } => {
            let content = match fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            match parse_packet(&content) {
                Ok(packet) => {
                    println!("{:#?}", packet);
                }
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Validate { files, host } => {
            let config_dir = host.join(".pidgin");
            let workflow_path = config_dir.join("WORKFLOW_REGISTRY.yaml");

            let workflows = match load_workflow_registry(&workflow_path) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Error loading workflow registry: {}", e);
                    std::process::exit(4);
                }
            };

            let mut all_passed = true;

            for file in &files {
                let content = match fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{}: FAIL (read error: {})", file.display(), e);
                        all_passed = false;
                        continue;
                    }
                };

                let packet = match parse_packet(&content) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("{}: FAIL (parse error: {})", file.display(), e);
                        all_passed = false;
                        continue;
                    }
                };

                let syntax_errors = validate_syntax(&packet);
                let schema_errors = validate_schema(&packet, &workflows);

                let mut file_errors = syntax_errors;
                file_errors.extend(schema_errors);

                if file_errors.is_empty() {
                    println!("{}: PASS", file.display());
                } else {
                    eprintln!("{}: FAIL", file.display());
                    for err in &file_errors {
                        eprintln!("  [{}] {}", err.code, err.message);
                    }
                    all_passed = false;
                }
            }

            if !all_passed {
                std::process::exit(1);
            }
        }

        Commands::Resolve { file, host } => {
            let config_dir = host.join(".pidgin");
            let workflow_path = config_dir.join("WORKFLOW_REGISTRY.yaml");
            let aliases_path = config_dir.join("REFERENCE_ALIASES.yaml");

            let workflows = match load_workflow_registry(&workflow_path) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Error loading workflow registry: {}", e);
                    std::process::exit(4);
                }
            };

            let aliases = match load_aliases(&aliases_path) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Error loading reference aliases: {}", e);
                    std::process::exit(4);
                }
            };

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

            // Get required inputs from the workflow, if known
            let required_inputs = packet
                .fields
                .get("wf")
                .and_then(|v| match v {
                    pidgin_core::ast::FieldValue::Scalar(s) => workflows.workflows.get(s),
                    _ => None,
                })
                .map(|w| w.required_inputs.clone())
                .unwrap_or_default();

            let ctx = ResolverContext {
                host_root: host.canonicalize().unwrap_or_else(|_| host.clone()),
                aliases,
                required_inputs,
            };

            let results = resolve_all(&packet, &ctx);

            if results.is_empty() {
                println!("{}: no references found", file.display());
                return;
            }

            let mut all_resolved_or_missing = true;
            println!("{}: {}", file.display(), results.len());

            for r in &results {
                let status = match r.status {
                    pidgin_core::resolver::ResolutionStatus::Resolved => "RESOLVED",
                    pidgin_core::resolver::ResolutionStatus::Missing => "MISSING",
                    pidgin_core::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
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
                if matches!(r.status, pidgin_core::resolver::ResolutionStatus::Unresolved) && r.required
                {
                    all_resolved_or_missing = false;
                }
            }

            if !all_resolved_or_missing {
                std::process::exit(3);
            }
        }

        Commands::Check { file, host } => {
            let config_dir = host.join(".pidgin");
            let workflow_path = config_dir.join("WORKFLOW_REGISTRY.yaml");
            let action_path = config_dir.join("ACTION_REGISTRY.yaml");
            let safety_path = config_dir.join("SAFETY_RULES.yaml");

            let workflows = match load_workflow_registry(&workflow_path) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Error loading workflow registry: {}", e);
                    std::process::exit(4);
                }
            };

            let actions = match load_action_registry(&action_path) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Error loading action registry: {}", e);
                    std::process::exit(4);
                }
            };

            let safety_rules = match load_safety_rules(&safety_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading safety rules: {}", e);
                    std::process::exit(4);
                }
            };

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
            let schema_errors = validate_schema(&packet, &workflows);

            let mut all_errors = Vec::new();
            for err in &syntax_errors {
                all_errors.push(format!("  [{}] {}", err.code, err.message));
            }
            for err in &schema_errors {
                all_errors.push(format!("  [{}] {}", err.code, err.message));
            }

            let safety_result = check_safety(&packet, &actions, &safety_rules, &workflows);
            for rule in &safety_result.fired_rules {
                all_errors.push(format!("  [{}] (safety)", rule));
            }

            if all_errors.is_empty() {
                println!("{}: PASS", file.display());
            } else {
                println!("{}: FAIL", file.display());
                for err in &all_errors {
                    eprintln!("{}", err);
                }
                let exit_code = if !syntax_errors.is_empty() || !schema_errors.is_empty() {
                    1
                } else {
                    2
                };
                std::process::exit(exit_code);
            }
        }
    }
}
