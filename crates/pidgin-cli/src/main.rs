use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use pidgin_core::context::build_context_plan;
use pidgin_core::expander::expand_to_run_packet;
use pidgin_core::logging::{log_event, LogEvent};
use pidgin_core::metrics::{compare_verbose, estimate_tokens, measure_packet};
use pidgin_core::parser::parse_packet;
use pidgin_core::registry::{load_action_registry, load_safety_rules, load_workflow_registry};
use pidgin_core::resolver::{load_aliases, resolve_all, ResolverContext};
use pidgin_core::router::{explain_route, route};
use pidgin_core::safety::check_safety;
use pidgin_core::validator::syntax::validate_syntax;
use pidgin_core::validator::schema::validate_schema;

fn load_pipeline_configs(host: &PathBuf) -> PipelineConfig {
    let config_dir = host.join(".pidgin");
    let workflow_path = config_dir.join("WORKFLOW_REGISTRY.yaml");
    let action_path = config_dir.join("ACTION_REGISTRY.yaml");
    let safety_path = config_dir.join("SAFETY_RULES.yaml");
    let aliases_path = config_dir.join("REFERENCE_ALIASES.yaml");

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
    let aliases = match load_aliases(&aliases_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error loading reference aliases: {}", e);
            std::process::exit(4);
        }
    };

    PipelineConfig {
        workflows,
        actions,
        safety_rules,
        aliases,
    }
}

struct PipelineConfig {
    workflows: pidgin_core::registry::WorkflowRegistry,
    actions: pidgin_core::registry::ActionRegistry,
    safety_rules: pidgin_core::registry::SafetyRules,
    aliases: pidgin_core::resolver::ReferenceAliases,
}

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
        file: PathBuf,
    },

    /// Validate a Pidgin packet (syntax + schema)
    Validate {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Validate → safety gate → resolve, end to end
    Check {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Resolve all short references in a packet
    Resolve {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Expand a packet into its executable form
    Expand {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        r#out: Option<PathBuf>,
    },

    /// Build a context plan for what to retrieve
    ContextPlan {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Estimate token cost of a packet
    Measure {
        file: PathBuf,
    },

    /// Compare a pgn file against a verbose text version
    Compare {
        pgn_file: PathBuf,
        #[arg(long)]
        verbose: PathBuf,
    },

    /// Run the full pipeline (parse → validate → safety → resolve → expand)
    Run {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Check host configuration
    Doctor {
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
                Ok(packet) => println!("{:#?}", packet),
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Validate { files, host } => {
            let cfg = load_pipeline_configs(&host);
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
                let mut errors = validate_syntax(&packet);
                errors.extend(validate_schema(&packet, &cfg.workflows));
                if errors.is_empty() {
                    println!("{}: PASS", file.display());
                } else {
                    eprintln!("{}: FAIL", file.display());
                    for err in &errors {
                        eprintln!("  [{}] {}", err.code, err.message);
                    }
                    all_passed = false;
                }
            }
            if !all_passed {
                std::process::exit(1);
            }
        }

        Commands::Check { file, host } => {
            let cfg = load_pipeline_configs(&host);
            let host_root = host.canonicalize().unwrap_or_else(|_| host.clone());
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

            let mut all_errors: Vec<String> = Vec::new();
            for err in &validate_syntax(&packet) {
                all_errors.push(format!("  [{}] {}", err.code, err.message));
            }
            for err in &validate_schema(&packet, &cfg.workflows) {
                all_errors.push(format!("  [{}] {}", err.code, err.message));
            }

            let safety_result = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
            for rule in &safety_result.fired_rules {
                all_errors.push(format!("  [{}] (safety)", rule));
            }

            let required_inputs = packet
                .fields
                .get("wf")
                .and_then(|v| match v {
                    pidgin_core::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s),
                    _ => None,
                })
                .map(|w| w.required_inputs.clone())
                .unwrap_or_default();

            let ctx = ResolverContext {
                host_root,
                aliases: cfg.aliases,
                required_inputs,
            };
            let resolved = resolve_all(&packet, &ctx);
            for r in &resolved {
                if matches!(r.status, pidgin_core::resolver::ResolutionStatus::Unresolved) && r.required {
                    all_errors.push(format!("  [UNRESOLVED] {} (required)", r.original));
                }
            }

            if all_errors.is_empty() {
                println!("{}: PASS", file.display());
            } else {
                println!("{}: FAIL", file.display());
                for err in &all_errors {
                    eprintln!("{}", err);
                }
                let has_validation = !validate_syntax(&packet).is_empty()
                    || !validate_schema(&packet, &cfg.workflows).is_empty();
                std::process::exit(if has_validation { 1 } else { 2 });
            }
        }

        Commands::Resolve { file, host } => {
            let cfg = load_pipeline_configs(&host);
            let host_root = host.canonicalize().unwrap_or_else(|_| host.clone());
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
            let required_inputs = packet
                .fields
                .get("wf")
                .and_then(|v| match v {
                    pidgin_core::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s),
                    _ => None,
                })
                .map(|w| w.required_inputs.clone())
                .unwrap_or_default();
            let ctx = ResolverContext {
                host_root,
                aliases: cfg.aliases,
                required_inputs,
            };
            let results = resolve_all(&packet, &ctx);
            if results.is_empty() {
                println!("{}: no references found", file.display());
                return;
            }
            let mut all_ok = true;
            println!("{}: {}", file.display(), results.len());
            for r in &results {
                let status = match r.status {
                    pidgin_core::resolver::ResolutionStatus::Resolved => "RESOLVED",
                    pidgin_core::resolver::ResolutionStatus::Missing => "MISSING",
                    pidgin_core::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
                };
                let path = r.resolved_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "-".to_string());
                println!("  {}  ns={}  id={}  confidence={:.1}  required={}  path={}", status, r.namespace, r.ref_id, r.confidence, r.required, path);
                if matches!(r.status, pidgin_core::resolver::ResolutionStatus::Unresolved) && r.required {
                    all_ok = false;
                }
            }
            if !all_ok {
                std::process::exit(3);
            }
        }

        Commands::Expand { file, host, r#out } => {
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

            let _ = log_event(
                &host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"),
                &LogEvent::Expand {
                    run_id: packet.run_id.clone(),
                    packet_type: "run".to_string(),
                },
            );
        }

        Commands::ContextPlan { file, host } => {
            let cfg = load_pipeline_configs(&host);
            let host_root = host.canonicalize().unwrap_or_else(|_| host.clone());
            let content = fs::read_to_string(&file).unwrap_or_else(|e| {
                eprintln!("{}: Error reading file: {}", file.display(), e);
                std::process::exit(1);
            });
            let packet = parse_packet(&content).unwrap_or_else(|e| {
                eprintln!("{}: Parse error: {}", file.display(), e);
                std::process::exit(1);
            });
            let required_inputs = packet
                .fields
                .get("wf")
                .and_then(|v| match v {
                    pidgin_core::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s),
                    _ => None,
                })
                .map(|w| w.required_inputs.clone())
                .unwrap_or_default();
            let ctx = ResolverContext { host_root, aliases: cfg.aliases, required_inputs };
            let resolved = resolve_all(&packet, &ctx);
            let plan = build_context_plan(&packet, &resolved);
            let yaml = serde_yaml::to_string(&plan).unwrap_or_else(|e| {
                eprintln!("Error serializing context plan: {}", e);
                std::process::exit(5);
            });
            println!("{}", yaml.trim());
        }

        Commands::Measure { file } => {
            let content = fs::read_to_string(&file).unwrap_or_else(|e| {
                eprintln!("{}: Error reading file: {}", file.display(), e);
                std::process::exit(1);
            });
            let tokens = estimate_tokens(&content);
            match parse_packet(&content) {
                Ok(packet) => {
                    let report = measure_packet(&packet);
                    let yaml = serde_yaml::to_string(&report).unwrap_or_else(|e| {
                        eprintln!("Error: {}", e);
                        std::process::exit(5);
                    });
                    println!("{}", yaml.trim());
                }
                Err(_) => {
                    println!("char_count: {}", content.len());
                    println!("estimated_tokens: {}", tokens);
                }
            }
        }

        Commands::Compare { pgn_file, verbose } => {
            let pgn_text = fs::read_to_string(&pgn_file).unwrap_or_else(|e| {
                eprintln!("Error reading pgn file: {}", e);
                std::process::exit(1);
            });
            let verbose_text = fs::read_to_string(&verbose).unwrap_or_else(|e| {
                eprintln!("Error reading verbose file: {}", e);
                std::process::exit(1);
            });
            let report = compare_verbose(&pgn_text, &verbose_text);
            let yaml = serde_yaml::to_string(&report).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(5);
            });
            println!("{}", yaml.trim());
        }

        Commands::Run { file, host, out } => {
            let cfg = load_pipeline_configs(&host);
            let host_root = host.canonicalize().unwrap_or_else(|_| host.clone());
            let run_id;

            // Parse
            let content = fs::read_to_string(&file).unwrap_or_else(|e| {
                eprintln!("{}: Error reading file: {}", file.display(), e);
                std::process::exit(1);
            });
            let packet = match parse_packet(&content) {
                Ok(p) => { run_id = p.run_id.clone(); p }
                Err(e) => {
                    eprintln!("{}: Parse error: {}", file.display(), e);
                    let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Parse { run_id: file.display().to_string(), ok: false });
                    std::process::exit(1);
                }
            };
            let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Parse { run_id: run_id.clone(), ok: true });

            // Validate
            let syntax_errors = validate_syntax(&packet);
            let schema_errors = validate_schema(&packet, &cfg.workflows);
            if !syntax_errors.is_empty() || !schema_errors.is_empty() {
                eprintln!("{}: Validation errors", file.display());
                for err in &syntax_errors { eprintln!("  [{}] {}", err.code, err.message); }
                for err in &schema_errors { eprintln!("  [{}] {}", err.code, err.message); }
                let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Validate { run_id: run_id.clone(), ok: false });
                std::process::exit(1);
            }
            let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Validate { run_id: run_id.clone(), ok: true });

            // Safety
            let safety = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
            let rules_str = safety.fired_rules.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",");
            let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::SafetyGate { run_id: run_id.clone(), blocked: safety.blocked, rules: rules_str });
            if safety.blocked {
                eprintln!("{}: Blocked by safety", file.display());
                for rule in &safety.fired_rules { eprintln!("  [{}]", rule); }
                std::process::exit(2);
            }

            // Resolve
            let required_inputs = packet
                .fields.get("wf")
                .and_then(|v| match v { pidgin_core::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s), _ => None })
                .map(|w| w.required_inputs.clone())
                .unwrap_or_default();
            let ctx = ResolverContext { host_root, aliases: cfg.aliases, required_inputs };
            let resolved = resolve_all(&packet, &ctx);
            let unresolved = resolved.iter().filter(|r| matches!(r.status, pidgin_core::resolver::ResolutionStatus::Unresolved)).count();
            let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Resolve { run_id: run_id.clone(), refs_total: resolved.len(), refs_unresolved: unresolved });
            for r in &resolved {
                if matches!(r.status, pidgin_core::resolver::ResolutionStatus::Unresolved) && r.required {
                    eprintln!("{}: Required reference unresolved: {}", file.display(), r.original);
                    std::process::exit(3);
                }
            }

            // Route
            let decision = route(&packet, &cfg.workflows, &safety);

            // Expand
            let expanded = expand_to_run_packet(&packet, &resolved, &safety, &cfg.workflows);
            let yaml = serde_yaml::to_string(&expanded).unwrap_or_else(|e| {
                eprintln!("Error serializing: {}", e);
                std::process::exit(5);
            });
            let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Expand { run_id: run_id.clone(), packet_type: "run".to_string() });

            match out {
                Some(path) => {
                    fs::write(&path, &yaml).unwrap_or_else(|e| {
                        eprintln!("Error writing output: {}", e);
                        std::process::exit(5);
                    });
                    println!("{}: expanded -> {} (dry-run)", file.display(), path.display());
                }
                None => {
                    println!("---");
                    println!("{}", yaml.trim());
                    println!("---");
                    println!("Route: {}", explain_route(&decision));
                }
            }

            let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Run { run_id: run_id.clone(), status: "dry_run_ok".to_string() });
        }

        Commands::Doctor { host } => {
            let config_dir = host.join(".pidgin");
            let checks = vec![
                ("WORKFLOW_REGISTRY.yaml", config_dir.join("WORKFLOW_REGISTRY.yaml")),
                ("ACTION_REGISTRY.yaml", config_dir.join("ACTION_REGISTRY.yaml")),
                ("SAFETY_RULES.yaml", config_dir.join("SAFETY_RULES.yaml")),
                ("REFERENCE_ALIASES.yaml", config_dir.join("REFERENCE_ALIASES.yaml")),
            ];

            let mut all_ok = true;
            for (name, path) in &checks {
                if path.exists() {
                    println!("  OK  {}", name);
                } else {
                    eprintln!("  MISS  {}", name);
                    all_ok = false;
                }
            }

            // Check YAML parsability
            for (name, path) in &checks {
                if path.exists() {
                    match std::fs::read_to_string(path) {
                        Ok(content) => {
                            if serde_yaml::from_str::<serde_yaml::Value>(&content).is_ok() {
                                println!("  OK  {} (valid YAML)", name);
                            } else {
                                eprintln!("  INVALID  {} (malformed YAML)", name);
                                all_ok = false;
                            }
                        }
                        Err(e) => {
                            eprintln!("  ERR  {}: {}", name, e);
                            all_ok = false;
                        }
                    }
                }
            }

            if all_ok {
                println!("All checks passed");
            } else {
                std::process::exit(4);
            }
        }
    }
}
