use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use pidgin_lang::context::build_context_plan;
use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::logging::{log_event, LogEvent};
use pidgin_lang::metrics::{compare_verbose, estimate_tokens, measure_packet};
use pidgin_lang::parser::parse_packet;
use pidgin_lang::registry::{load_action_registry, load_safety_rules, load_workflow_registry};
use pidgin_lang::resolver::{load_aliases, resolve_all, ResolverContext};
use pidgin_lang::router::{explain_route, route};
use pidgin_lang::safety::{check_resolved_refs_safety, check_safety};
use pidgin_lang::validator::syntax::validate_syntax;
use pidgin_lang::validator::schema::validate_schema;

fn canonicalize_host(host: &Path) -> PathBuf {
    host.canonicalize().unwrap_or_else(|e| {
        eprintln!("error: cannot canonicalize host path {}: {}", host.display(), e);
        std::process::exit(1);
    })
}

fn load_pipeline_configs(host: &Path) -> PipelineConfig {
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
    workflows: pidgin_lang::registry::WorkflowRegistry,
    actions: pidgin_lang::registry::ActionRegistry,
    safety_rules: pidgin_lang::registry::SafetyRules,
    aliases: pidgin_lang::resolver::ReferenceAliases,
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

    /// Scaffold a default .pidgin/ config directory
    Init {
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        force: bool,
    },

    /// Print full documentation for agents (grammar, CLI, safety, integration)
    Docs {
        #[arg(long, default_value = "markdown")]
        format: String,
    },
}

const DEFAULT_RUNTIME_CONFIG: &str = r#"runtime:
  name: pidgin
  spec_version: "1.0"
  strict_mode: true
  default_dry_run: true

host:
  root: "."
  inbox: ".pidgin/inbox"
  outbox: ".pidgin/generated"
  logs: ".pidgin/logs"
  config_dir: ".pidgin"

paths:
  aliases: .pidgin/REFERENCE_ALIASES.yaml
  workflow_registry: .pidgin/WORKFLOW_REGISTRY.yaml
  action_registry: .pidgin/ACTION_REGISTRY.yaml
  output_registry: .pidgin/OUTPUT_REGISTRY.yaml
  safety_rules: .pidgin/SAFETY_RULES.yaml
  token_budgets: .pidgin/TOKEN_BUDGETS.yaml

logs:
  agent_messages: .pidgin/logs/AGENT_MESSAGES.csv
  protocol_errors: .pidgin/logs/PROTOCOL_ERRORS.csv
  runtime_runs: .pidgin/logs/PIDGIN_RUNTIME_RUNS.csv
  token_usage: .pidgin/logs/TOKEN_USAGE_LOG.csv

defaults:
  deny:
    - publish
    - send
    - delete
    - secrets
    - credentials
    - external_action
  human_for_dangerous_actions: true
  block_private_paths: true
  estimate_tokens_by_chars: true
"#;

const DEFAULT_WORKFLOW_REGISTRY: &str = r#"workflows:
  generic_review:
    description: Review a piece of content or code against a set of source references.
    risk_default: med
    allowed_modes: [draft, review, approval]
    required_inputs: [primary_subject, source_refs]
    expected_outputs: [review_notes, risk_flags, approval]
    recommended_executor: claude-code
    fallback_executor: opencode

  generic_health_check:
    description: Check a host's structure, configuration, and logs for drift or errors.
    risk_default: low
    allowed_modes: [review, patch]
    required_inputs: [host_tree, configs, logs]
    expected_outputs: [health_report, review_required]
    recommended_executor: opencode
    fallback_executor: claude-code

  generic_draft_and_distribute:
    description: Draft a piece of output content from a source and prepare it for
      multiple destination formats, gated on human approval before anything is sent.
    risk_default: med
    allowed_modes: [draft, review, approval]
    required_inputs: [source, style_guide]
    expected_outputs: [drafts, approval]
    recommended_executor: claude-code
    fallback_executor: codex
"#;

const DEFAULT_ACTION_REGISTRY: &str = r#"safe:
  - read
  - retrieve
  - summarize
  - classify
  - draft
  - review
  - score
  - rank
  - flag
  - compare
  - extract
  - package
  - validate
  - log
  - index

controlled:
  - patch
  - move
  - rename
  - update
  - append
  - reindex
  - optimize
  - compress
  - expand
  - rerank

human_gated:
  - publish
  - send
  - delete
  - moderate
  - archive
  - credential
  - approve
  - reject
  - promote_memory
  - change_policy
  - external_action
"#;

const DEFAULT_SAFETY_RULES: &str = r#"default_deny:
  - publish
  - send
  - delete
  - secrets
  - credentials
  - external_action

private_paths:
  - ".env"
  - ".env.*"
  - "*.key"
  - "*.pem"
  - ".git/"
  - "**/secrets/**"
  - "**/.ssh/**"

human_required:
  actions:
    - publish
    - send
    - delete
    - moderate
    - credential
    - promote_memory
    - external_action
  risk_levels:
    - high
    - crit

block_if:
  action_in_do_and_deny: true
  private_path_referenced: true
  unknown_workflow: true
  invalid_mode: true
  missing_required_field: true
  dangerous_action_without_human: true
"#;

const DEFAULT_REFERENCE_ALIASES: &str = r#"aliases: {}
common: {}
"#;

const DOCS: &str = r#"# Pidgin — Agent Handoff Protocol & Runtime

Pidgin is a compact, local-first protocol for structured handoffs between agents,
between an agent and a human operator, and between an orchestrator and executors.

## Architecture

The pipeline is: parse → validate (syntax + schema) → safety gate → resolve references
→ expand to executable packet → context plan → token estimate → route → log.

Every stage is a pure function. The safety gate enforces 9 rules (SG-1 through SG-9).
The resolver resolves short references (namespace:id or bare aliases) into real paths/IDs
with containment and symlink traversal protection.

## Packet Grammar

A packet is a header line + key=value fields:

  @run task.example
  wf=generic_review
  mode=draft
  in=[primary_subject,source_refs]
  out=[review_notes]
  do=[draft,review]
  deny=[publish,send,delete,secrets]
  risk=med
  human=yes

Directives: @run, @result, @approval, @context
Scalars: key=value
Lists: key=[val1,val2]
Quoted strings: key="value with spaces"
Comments: # line comment (full line only)

Reference syntax:
  namespace:id     — e.g. file:src/main.rs, ep:UNIT012, workflow:generic_review
  bare_alias       — resolved through REFERENCE_ALIASES.yaml

## CLI Commands

- init [--host PATH] [--force]     — Scaffold .pidgin/ config directory
- parse FILE                       — Parse a packet, print AST
- validate FILES... --host PATH    — Validate syntax + schema
- check FILE --host PATH           — Parse → validate → safety → resolve
- resolve FILE --host PATH         — Resolve all short references
- expand FILE --host PATH [--out F] — Expand to executable YAML
- context-plan FILE --host PATH    — Build context retrieval plan
- measure FILE                     — Estimate token cost
- compare --pgn F --verbose F      — Compare Pidgin vs verbose token cost
- run FILE --host PATH [--out F]   — Full pipeline end-to-end
- doctor --host PATH               — Check host configuration
- docs [--format markdown]         — Print this documentation

## Safety Rules

SG-1: Action in both do and deny → blocked
SG-2: Human-gated action without human=yes → blocked
SG-3: High/critical risk forces human=yes, cannot override
SG-4: References resolving to private paths → blocked
SG-5: Unknown workflow → blocked
SG-6: Invalid mode → blocked
SG-7: Free-text note field is never parsed for instructions
SG-8: Unresolved required inputs → expansion blocked
SG-9: Critical risk requires an @approval packet

Principle: Fail closed — if uncertain, block.

## Configuration (.pidgin/)

- PIDGIN_RUNTIME_CONFIG.yaml — runtime settings
- WORKFLOW_REGISTRY.yaml     — workflow definitions
- ACTION_REGISTRY.yaml       — action tiers (safe/controlled/human_gated)
- SAFETY_RULES.yaml          — deny list, private paths, human rules
- REFERENCE_ALIASES.yaml     — short-name aliases for references

## Multi-Agent Integration

  Agent A ──.pgn──→ Pidgin (validate→safety→resolve→expand) ──→ Agent B
                                                                    │
                                                               result .pgn
                                                                    │
  Agent A ←──────────────────── reads result ───────────────────────┘

- LangGraph: Pidgin node between graph steps, expanded packets as state messages
- CrewAI: Agent task outputs as .pgn, Pidgin validates handoffs
- A2A: Expanded Run Packet as A2A Task payload
- MCP: Pidgin as an MCP server exposing parse/validate/expand tools
- Python SDK: pydantic-typed wrapper for Python orchestrators

## Library Usage

  use pidgin_lang::parser::parse_packet;
  use pidgin_lang::safety::check_safety;
  use pidgin_lang::expander::expand_to_run_packet;

  let packet = parse_packet("@run my.task\nwf=generic_review\nmode=draft")
      .expect("valid packet");

Modules: parser, lexer, ast, validator::syntax, validator::schema, safety,
resolver, expander, context, metrics, router, logging, registry, errors.
"#;

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
                    pidgin_lang::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s),
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
            let resolved_fired = check_resolved_refs_safety(&resolved, &cfg.safety_rules.private_paths);
            for rule in &resolved_fired {
                all_errors.push(format!("  [{}] (safety after resolution)", rule));
            }
            for r in &resolved {
                if r.required && matches!(r.status, pidgin_lang::resolver::ResolutionStatus::Unresolved | pidgin_lang::resolver::ResolutionStatus::Forbidden) {
                    let label = match r.status {
                        pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
                        pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
                        _ => "ERROR",
                    };
                    all_errors.push(format!("  [{}] {} (required)", label, r.original));
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
            let required_inputs = packet
                .fields
                .get("wf")
                .and_then(|v| match v {
                    pidgin_lang::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s),
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
                    pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
                    pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
                    pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
                    pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
                };
                let path = r.resolved_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "-".to_string());
                println!("  {}  ns={}  id={}  confidence={:.1}  required={}  path={}", status, r.namespace, r.ref_id, r.confidence, r.required, path);
                if r.required && matches!(r.status, pidgin_lang::resolver::ResolutionStatus::Unresolved | pidgin_lang::resolver::ResolutionStatus::Forbidden) {
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
            let host_root = canonicalize_host(&host);
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
                    pidgin_lang::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s),
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
            let host_root = canonicalize_host(&host);
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
                .and_then(|v| match v { pidgin_lang::ast::FieldValue::Scalar(s) => cfg.workflows.workflows.get(s), _ => None })
                .map(|w| w.required_inputs.clone())
                .unwrap_or_default();
            let ctx = ResolverContext { host_root, aliases: cfg.aliases, required_inputs };
            let resolved = resolve_all(&packet, &ctx);

            // Post-resolution private path check
            let resolved_fired = check_resolved_refs_safety(&resolved, &cfg.safety_rules.private_paths);
            if !resolved_fired.is_empty() {
                for rule in &resolved_fired {
                    eprintln!("{}: Blocked by safety after resolution: {}", file.display(), rule);
                }
                std::process::exit(2);
            }

            let unresolved = resolved.iter().filter(|r| matches!(r.status, pidgin_lang::resolver::ResolutionStatus::Unresolved | pidgin_lang::resolver::ResolutionStatus::Forbidden)).count();
            let _ = log_event(&host.join(".pidgin").join("logs").join("PIDGIN_RUNTIME_RUNS.csv"), &LogEvent::Resolve { run_id: run_id.clone(), refs_total: resolved.len(), refs_unresolved: unresolved });
            for r in &resolved {
                if r.required && matches!(r.status, pidgin_lang::resolver::ResolutionStatus::Unresolved | pidgin_lang::resolver::ResolutionStatus::Forbidden) {
                    eprintln!("{}: Required reference {}: {}", file.display(), match r.status {
                        pidgin_lang::resolver::ResolutionStatus::Forbidden => "forbidden (path traversal blocked)",
                        _ => "unresolved",
                    }, r.original);
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

        Commands::Init { host, force } => {
            let config_dir = host.join(".pidgin");
            let logs_dir = config_dir.join("logs");

            if config_dir.exists() && !force {
                eprintln!("{} already exists (use --force to overwrite)", config_dir.display());
                std::process::exit(1);
            }

            fs::create_dir_all(&logs_dir).unwrap_or_else(|e| {
                eprintln!("error: cannot create {}: {}", logs_dir.display(), e);
                std::process::exit(1);
            });

            let files: Vec<(&str, &Path, &str)> = vec![
                ("PIDGIN_RUNTIME_CONFIG.yaml", &config_dir, DEFAULT_RUNTIME_CONFIG),
                ("WORKFLOW_REGISTRY.yaml", &config_dir, DEFAULT_WORKFLOW_REGISTRY),
                ("ACTION_REGISTRY.yaml", &config_dir, DEFAULT_ACTION_REGISTRY),
                ("SAFETY_RULES.yaml", &config_dir, DEFAULT_SAFETY_RULES),
                ("REFERENCE_ALIASES.yaml", &config_dir, DEFAULT_REFERENCE_ALIASES),
            ];

            let mut count = 0;
            for (name, dir, content) in &files {
                let path = dir.join(name);
                if path.exists() && !force {
                    println!("  SKIP {}", name);
                    continue;
                }
                fs::write(&path, content).unwrap_or_else(|e| {
                    eprintln!("error: cannot write {}: {}", path.display(), e);
                    std::process::exit(1);
                });
                println!("  CREATE {}", name);
                count += 1;
            }

            if count > 0 {
                println!("initialized pidgin host config in {}", config_dir.display());
            } else {
                println!("nothing to do (use --force to overwrite existing files)");
            }
        }

        Commands::Docs { format } => {
            match format.as_str() {
                "markdown" => println!("{}", DOCS.trim()),
                "json" => {
                    // TODO: structured JSON output
                    println!("{}", DOCS.trim());
                }
                other => {
                    eprintln!("unsupported format '{}' (try \"markdown\")", other);
                    std::process::exit(1);
                }
            }
        }
    }
}
