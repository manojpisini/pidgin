use std::fs;
use std::path::{Path, PathBuf};

use pidgin_lang::ast::{FieldValue, PgnPacket};
use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::logging::{LogEvent, log_event};
use pidgin_lang::parser::parse_packet;
use pidgin_lang::registry::{ActionRegistry, SafetyRules, WorkflowRegistry};
use pidgin_lang::resolver::{ReferenceAliases, ResolverContext, load_aliases, resolve_all};
use pidgin_lang::safety::{check_resolved_refs_safety, check_safety};
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

pub fn canonicalize_host(host: &Path) -> PathBuf {
    host.canonicalize().unwrap_or_else(|e| {
        eprintln!(
            "error: cannot canonicalize host path {}: {}",
            host.display(),
            e
        );
        std::process::exit(1);
    })
}

pub fn load_pipeline_configs(host: &Path) -> PipelineConfig {
    let config_dir = host.join(".pidgin");
    PipelineConfig {
        workflows: load_or_exit(
            pidgin_lang::registry::load_workflow_registry(
                &config_dir.join("WORKFLOW_REGISTRY.yaml"),
            ),
            "workflow registry",
        ),
        actions: load_or_exit(
            pidgin_lang::registry::load_action_registry(&config_dir.join("ACTION_REGISTRY.yaml")),
            "action registry",
        ),
        safety_rules: load_or_exit(
            pidgin_lang::registry::load_safety_rules(&config_dir.join("SAFETY_RULES.yaml")),
            "safety rules",
        ),
        aliases: load_or_exit(
            load_aliases(&config_dir.join("REFERENCE_ALIASES.yaml")),
            "reference aliases",
        ),
    }
}

pub struct PipelineConfig {
    pub workflows: WorkflowRegistry,
    pub actions: ActionRegistry,
    pub safety_rules: SafetyRules,
    pub aliases: ReferenceAliases,
}

pub fn load_or_exit<T>(result: Result<T, impl std::fmt::Display>, label: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error loading {}: {}", label, e);
            std::process::exit(4);
        }
    }
}

pub fn get_required_inputs(packet: &PgnPacket, workflows: &WorkflowRegistry) -> Vec<String> {
    packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => workflows.workflows.get(s),
            _ => None,
        })
        .map(|w| w.required_inputs.clone())
        .unwrap_or_default()
}

pub fn process_packet(
    path: &Path,
    cfg: &PipelineConfig,
    host_root: &Path,
    outbox: &Path,
    config_dir: &Path,
) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  error reading: {}", e);
            return;
        }
    };
    let packet = match parse_packet(&content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  parse error: {}", e);
            let _ = log_event(
                &config_dir.join("logs").join("PROTOCOL_ERRORS.csv"),
                &LogEvent::Parse {
                    run_id: path.display().to_string(),
                    ok: false,
                },
            );
            return;
        }
    };

    let mut valid = true;
    for err in &validate_syntax(&packet) {
        eprintln!("  [{}] {}", err.code, err.message);
        valid = false;
    }
    for err in &validate_schema(&packet, &cfg.workflows) {
        eprintln!("  [{}] {}", err.code, err.message);
        valid = false;
    }
    if !valid {
        let _ = log_event(
            &config_dir.join("logs").join("PROTOCOL_ERRORS.csv"),
            &LogEvent::Validate {
                run_id: packet.run_id.clone(),
                ok: false,
            },
        );
        return;
    }

    let safety = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
    if safety.blocked {
        eprintln!("  blocked by safety:");
        for rule in &safety.fired_rules {
            eprintln!("    [{}]", rule);
        }
        let _ = log_event(
            &config_dir.join("logs").join("PROTOCOL_ERRORS.csv"),
            &LogEvent::SafetyGate {
                run_id: packet.run_id.clone(),
                blocked: true,
                rules: safety
                    .fired_rules
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            },
        );
        return;
    }

    let required_inputs = get_required_inputs(&packet, &cfg.workflows);
    let ctx = ResolverContext {
        host_root: host_root.to_path_buf(),
        aliases: cfg.aliases.clone(),
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);

    let resolved_fired = check_resolved_refs_safety(&resolved, &cfg.safety_rules.private_paths);
    if !resolved_fired.is_empty() {
        eprintln!("  blocked by safety after resolution:");
        for rule in &resolved_fired {
            eprintln!("    [{}]", rule);
        }
        return;
    }

    let expanded = expand_to_run_packet(&packet, &resolved, &safety, &cfg.workflows);
    let out_path = outbox.join(format!(
        "{}.RUN_PACKET.yaml",
        packet.run_id.replace('.', "_")
    ));
    let yaml = match serde_yaml::to_string(&expanded) {
        Ok(y) => y,
        Err(e) => {
            eprintln!("  error serializing: {}", e);
            return;
        }
    };
    match fs::write(&out_path, &yaml) {
        Ok(_) => eprintln!("  -> {}", out_path.display()),
        Err(e) => eprintln!("  error writing: {}", e),
    }

    let _ = log_event(
        &config_dir.join("logs").join("PIDGIN_RUNTIME_RUNS.csv"),
        &LogEvent::Run {
            run_id: packet.run_id.clone(),
            status: "watch_ok".to_string(),
        },
    );
}

pub const DEFAULT_WORKFLOW_REGISTRY: &str = r#"workflows:
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

pub const DEFAULT_ACTION_REGISTRY: &str = r#"safe:
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

pub const DEFAULT_SAFETY_RULES: &str = r#"private_paths:
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
"#;

pub const DEFAULT_REFERENCE_ALIASES: &str = r#"aliases: {}
common: {}
"#;

pub mod check;
pub mod context_plan;
pub mod docs;
pub mod doctor;
pub mod expand;
pub mod init;
pub mod measure;
pub mod parse;
pub mod resolve;
pub mod run;
#[cfg(feature = "server")]
pub mod serve;
pub mod validate;
pub mod watch;
