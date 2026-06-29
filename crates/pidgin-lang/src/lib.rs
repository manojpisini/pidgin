//! # Pidgin — A Compact Agent Handoff Protocol & Runtime
//!
//! Pidgin is a minimal, local-first protocol runtime for structured, validated
//! handoffs between AI agents. It parses a compact nine-line text packet, runs
//! it through a deterministic pipeline (validate → safety check → resolve →
//! expand), and produces an executable YAML specification — all without calling
//! a single LLM or opening a single network socket.
//!
//! ```text
//! .pgn file (9 lines of key=value text)
//!     │
//!     ▼
//! ┌─────────────┐
//! │   Lexer     │  winnow tokenizer, input limits
//! └─────────────┘
//!     │
//!     ▼
//! ┌─────────────┐
//! │   Parser    │  typed AST (PgnPacket)
//! └─────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │ Syntax Validator  │  required fields present? types correct?
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │ Schema Validator  │  workflow/mode/risk legal against registries?
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │   Safety Gate     │  9 rules (SG-1 through SG-9), fail closed
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │ Reference Resolver │ short refs → canonical paths/IDs
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │  Packet Expander   │  fully-specified executable YAML
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │  Context Planner   │  what to retrieve and how
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │  Token Estimator    │  packet + context token cost
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │  Router Planner     │  recommended executor
//! └──────────────────┘
//!     │
//!     ▼
//! ┌──────────────────┐
//! │  Logger              │  every step writes structured log
//! └──────────────────┘
//!     │
//!     ▼
//! Expanded packet, ready for: dry-run report | execution | human approval queue
//! ```
//!
//! # Why This Exists
//!
//! Every serious study of multi-agent token cost in 2025–2026 converges on the
//! same finding: the communication layer is the dominant cost and failure
//! surface, not the reasoning layer. Agents pass verbose natural-language
//! messages to each other — paragraphs of implicit instructions, unclearly
//! scoped tasks, unvalidated assumptions. Each handoff costs hundreds or
//! thousands of tokens. Worse, there's no validation: Agent B can interpret
//! Agent A's message differently from what Agent A intended, and nobody audits
//! it until something breaks.
//!
//! Pidgin formalizes the narrow waist between agents into a typed, validated,
//! safety-checked wire format. The packet is nine lines. The entire runtime
//! pipeline — lex, parse, validate, safety, resolve, expand, log — runs in
//! single-digit milliseconds on a laptop. You can audit every handoff because
//! every handoff is logged to a structured file.
//!
//! Pidgin is not an orchestrator. It does not run agents, it does not make
//! decisions, it does not call models. It sits between whatever produces a task
//! (a LangGraph node, a CrewAI agent, a human typing at a terminal) and
//! whatever executes it (Claude, Codex, a shell script), and ensures the
//! handoff is parseable, safe, and logged.
//!
//! # Quick Start
//!
//! ```bash
//! cargo install pidgin-lang
//! pgn init                   # scaffold .pidgin/ config
//! pgn parse examples/basic/generic_task.pgn
//! pgn run examples/basic/generic_task.pgn
//! pgn check examples/basic/unsafe_contradiction.pgn
//! pgn measure examples/basic/generic_task.pgn
//! pgn doctor
//! ```
//!
//! # Packet Grammar
//!
//! A Pidgin packet is plain text with a strict, unambiguous grammar. There is
//! no nesting, no optional syntax, no inline markup — every byte is either a
//! header, a field, a list element, or a comment.
//!
//! ## Header
//!
//! ```text
//! @<directive> <run_id>
//! ```
//!
//! Four directives:
//!
//! | Directive | Purpose |
//! |-----------|---------|
//! | `@run` | A task for an agent to execute |
//! | `@result` | The outcome of a completed task |
//! | `@approval` | Human sign-off for a critical-risk task |
//! | `@context` | A request for additional information |
//!
//! `run_id` is a dotted identifier — `task.example`, `docs.review.2026-06-01` —
//! that uniquely identifies the handoff across the system.
//!
//! ## Fields
//!
//! Every field is `key=value` on its own line. Whitespace around `=` is not
//! allowed, which eliminates an entire class of lexer ambiguities.
//!
//! ```text
//! wf=generic_review          # bare scalar
//! risk=med                   # bare scalar (allowed values depend on workflow)
//! in=[primary_subject,refs]  # list — comma-separated, no spaces
//! note="Draft only"          # quoted string (for free text)
//! ```
//!
//! ### Scalar fields
//!
//! | Field | Type | Appears On |
//! |-------|------|------------|
//! | `wf` | bare word | all directives |
//! | `mode` | bare word | `@run`, `@context` |
//! | `risk` | `low │ med │ high │ crit` | `@run`, `@approval` |
//! | `human` | `yes │ no` | `@run`, `@approval` |
//! | `status` | `ok │ fail │ partial` | `@result`, `@approval` |
//! | `ttl` | integer | all directives |
//! | `note` | quoted string | all directives |
//!
//! ### List fields
//!
//! | Field | Appears On | Contents |
//! |-------|------------|----------|
//! | `in` | `@run`, `@context` | Input references |
//! | `out` | `@run`, `@result` | Output references |
//! | `do` | `@run` | Actions to perform |
//! | `deny` | `@run` | Actions to explicitly deny |
//! | `produced` | `@result` | References to produced artifacts |
//!
//! ## Reference syntax
//!
//! Inside list fields, references can be either:
//!
//! - **Namespaced:** `namespace:id` — e.g. `file:src/main.rs`, `ep:UNIT012`
//! - **Bare alias:** `primary_subject` — resolved through `REFERENCE_ALIASES.yaml`
//!
//! Built-in namespaces:
//!
//! | Namespace | What It References |
//! |-----------|-------------------|
//! | `ep` | Entity pointer — a content item, document, or record |
//! | `rb` | Rollback target — a point-in-time snapshot |
//! | `ledger` | Ledger entry — an audit record |
//! | `claim` | Claim — a config key, channel, or property |
//! | `policy` | Policy — a rule file or constraints document |
//! | `skill` | Skill — a capability or tool definition |
//! | `wf` | Workflow — a workflow definition |
//! | `file` | File path (checked against safety rules) |
//! | `folder` | Directory path (checked against safety rules) |
//! | `dash` | Dashboard — a view or report |
//! | `queue` | Queue — a named message queue |
//!
//! # Safety Gate (SG-1 through SG-9)
//!
//! The safety gate is the core safety mechanism. It is a separate pipeline stage
//! — it runs after parsing and validation but before reference resolution and
//! expansion. It cannot be disabled, skipped, or overridden by packet fields.
//!
//! Every rule fails closed: if the gate cannot determine whether a rule applies,
//! it treats the rule as violated.
//!
//! ## SG-1 — Contradiction
//!
//! An action cannot appear in both `do` and `deny`. If a packet says both
//! "do publish" and "deny publish", something is wrong. The runtime refuses
//! to guess which intent is correct.
//!
//! ## SG-2 — Missing Human Approval
//!
//! If a packet requests a human-gated action (publishing, deleting, sending
//! credentials, etc.) without `human=yes`, the gate blocks. Human-gated actions
//! are defined in `ACTION_REGISTRY.yaml` as the `human_gated` tier plus
//! `human_required_actions` in `SAFETY_RULES.yaml`.
//!
//! ## SG-3 — Forced Human Approval
//!
//! High- or critical-risk packets cannot opt out of human review. Even if the
//! packet declares `human=no`, the gate overrides it. This prevents a risky
//! packet from bypassing human oversight by lying about its own `human` field.
//!
//! ## SG-4 — Private Path Access
//!
//! Any `file:` or `folder:` reference that resolves to a path matching a
//! private path pattern (`.env`, `.ssh/`, `*.pem`, `secrets/`, etc.) is
//! blocked. Paths are canonicalized before matching, so symlink tricks and
//! traversal sequences (`../../etc/passwd`, `%2e%2e`) are caught.
//!
//! ## SG-5 — Unknown Workflow
//!
//! The `wf` field must match a workflow defined in `WORKFLOW_REGISTRY.yaml`.
//! An unknown workflow is not a no-op — it is a bug or an attack.
//!
//! ## SG-6 — Invalid Mode
//!
//! The `mode` field must be in the workflow's `allowed_modes` list. A
//! `generic_review` workflow should never be executed in `production` mode.
//!
//! ## SG-7 — Note Isolation
//!
//! The `note` field stores free text. No pipeline stage reads or interprets it.
//! This is structural: the note is an opaque string throughout the entire
//! pipeline. If you want an agent to read a note, the expanded packet makes
//! it available, but the runtime never acts on its contents. This closes
//! the most obvious prompt-injection surface.
//!
//! ## SG-8 — Unresolved Required Input
//!
//! If a required input reference fails to resolve (the alias is not in
//! `REFERENCE_ALIASES.yaml` and the namespace:id doesn't exist), expansion
//! is blocked. Running with missing inputs produces silent failures.
//!
//! ## SG-9 — Critical Risk Requires Approval Packet
//!
//! A packet with `risk=crit` requires a separate `@approval` packet with
//! `status=ok` before it can be expanded. A single `human=yes` on the same
//! packet is not enough — the two-packet pattern ensures separation of
//! concerns.
//!
//! # Multi-Agent Integration
//!
//! Pidgin is the handoff *format*, not the orchestrator. Here is how it fits
//! into various agent architectures:
//!
//! ```text
//!                                          expanded .pgn
//! Orchestrator ─── .pgn ──→ Pidgin ──────────────────────────→ Executor Agent
//! (LangGraph,    ←──────  (validate, safety,    ←──────────  (Claude, Codex,
//!  CrewAI,       result    resolve, expand,      result       shell, tool)
//!  A2A, MCP)               log)
//! ```
//!
//! ## LangGraph
//!
//! Add a Pidgin node between any two agent nodes. The Pidgin node parses and
//! validates the `.pgn` packet produced by the source agent, routes to the
//! destination agent based on the expanded packet, and logs the handoff to the
//! shared graph state. The flow remains typed and auditable.
//!
//! ## CrewAI
//!
//! Each CrewAI agent produces a `.pgn` packet as its task output. A custom
//! CrewAI tool wraps `pgn run` (or the library) to validate inter-agent
//! handoffs. Invalid or unsafe handoffs block before reaching the next agent.
//!
//! ## A2A (Agent2Agent)
//!
//! Pidgin's expanded Run Packet maps naturally into an A2A Task. The `.pgn`
//! format becomes the wire representation inside trust boundaries; the expanded
//! A2A Task is the representation crossing them. Pidgin's safety gate provides
//! the guardrails that A2A intentionally leaves to implementors.
//!
//! ## MCP (Model Context Protocol)
//!
//! Pidgin can run as an MCP server, exposing `parse`, `validate`, `check`,
//! and `expand` as MCP tools. Agents connected through MCP call these tools
//! as part of their workflow, getting structured, validated handoffs without
//! leaving the MCP protocol.
//!
//! ## Python SDK
//!
//! The Python SDK (scaffolded in `python/`) wraps the `pgn` binary via
//! subprocess, giving Python-based orchestrators (LangChain, CrewAI,
//! custom scripts) typed Pydantic models for packets, safety results, and
//! expanded output. PyO3 bindings are on the roadmap.
//!
//! # Host Configuration
//!
//! Every Pidgin host provides a `.pidgin/` directory with five YAML config
//! files. Run `pgn init` to scaffold default versions.
//!
//! | File | What It Defines |
//! |------|----------------|
//! | `PIDGIN_RUNTIME_CONFIG.yaml` | Host name, log directory, default deny list, input limits (1 MB max packet, 100 max fields, 10K max field length, 10 MB max config) |
//! | `WORKFLOW_REGISTRY.yaml` | Workflow definitions — each with risk default, allowed modes, required inputs, expected outputs, recommended executor, and human-approval requirement |
//! | `ACTION_REGISTRY.yaml` | Three tiers: `safe` (always allowed), `controlled` (allowed with validation), `human_gated` (requires `human=yes`) |
//! | `SAFETY_RULES.yaml` | Default deny list, gitignore-style private path patterns, human-required actions and risk levels |
//! | `REFERENCE_ALIASES.yaml` | Short-name aliases mapping bare identifiers to full `namespace:id` references |
//!
//! Config files are loaded at startup from the host root (`.` or
//! `$PIDGIN_ROOT_DIR`). Each file is validated for structure and key presence.
//!
//! # CLI
//!
//! The `pgn` binary exposes every pipeline stage as a subcommand plus
//! infrastructure commands:
//!
//! | Command | Action |
//! |---------|--------|
//! | `init` | Scaffold `.pidgin/` directory |
//! | `parse <path>` | Lex and parse, print AST |
//! | `validate <path>` | Syntax + schema validation |
//! | `check <path>` | Full guard: validate → safety → resolve |
//! | `resolve <path>` | Expand short references |
//! | `expand <path>` | Full pipeline → executable YAML |
//! | `run <path>` | Full pipeline + structured logging |
//! | `measure <path>` | Estimate token cost |
//! | `compare <path>` | Compare vs verbose token cost |
//! | `context-plan <path>` | Build retrieval plan |
//! | `doctor` | Check host configuration health |
//! | `docs` | Print full protocol documentation |
//!
//! Exit codes:
//! - `0` success
//! - `1` validation error (syntax or schema)
//! - `2` safety gate blocked
//! - `3` unresolved required reference
//! - `4` configuration error
//! - `5` internal error (file a bug)
//!
//! # Using as a Library
//!
//! Add to `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! pidgin-lang = "0.1"
//! ```
//!
//! Then compose your own pipeline:
//!
//! ```rust
//! use pidgin_lang::parser::parse_packet;
//! use pidgin_lang::safety::check_safety;
//! use pidgin_lang::expander::expand_to_run_packet;
//!
//! let packet = parse_packet("@run my.task\nwf=generic_review\nmode=draft")
//!     .expect("valid packet");
//! ```
//!
//! Every stage function is public and well-typed. You can:
//!
//! - Call only the parser (if you just need an AST)
//! - Call validate + safety without resolving (quick check)
//! - Skip the safety gate in test/development only
//! - Add custom stages before or after any built-in stage
//! - Implement your own logger by implementing the `Logger` trait
//!
//! # Safety Properties
//!
//! Beyond the safety gate, several cross-cutting properties hold:
//!
//! - **No network calls in core.** The runtime never opens a socket or makes
//!   an HTTP request. There is no network attack surface.
//! - **No LLM in the hot path.** Every stage is deterministic. No model is
//!   called at any point. The safety gate is algorithmic, not probabilistic.
//! - **Fail closed.** Every uncertain decision blocks. Unknown workflow →
//!   blocked. Missing required input → blocked. Ambiguous reference → blocked.
//! - **Path containment.** All file references are canonicalized and checked
//!   against the host root. No traversal, no symlink escape, no encoded bypass.
//! - **Input limits.** Packets are bounded (1 MB, 100 fields, 10K per field).
//!   Config files are bounded (10 MB). The lexer rejects oversized input before
//!   any heavy processing.
//! - **Sanitized logging.** User values in logs are filtered to printable ASCII
//!   with length caps. No log injection, no CSV injection.
//! - **Append-only logs.** Every log write completes (flush + close) before
//!   the next pipeline stage starts. No log loss on crash.
//!
//! # Modules
//!
//! The library is organized into modules corresponding to pipeline stages and
//! supporting infrastructure:

/// Typed AST types — `PgnPacket`, `FieldValue`, `PacketDirective`, and related
/// data structures representing a parsed Pidgin packet in memory.
pub mod ast;

/// Context planner — decides what information to retrieve based on a packet's
/// workflow and resolved references, producing a structured retrieval plan.
pub mod context;

/// Error types — `ParseError`, `ValidationError`, `SafetyError`, and other
/// error enums with typed variants, source chaining, and formatted messages.
pub mod errors;

/// Packet expander — takes a validated, safety-checked, resolved packet and
/// produces a fully-specified executable YAML packet ready for consumption by
/// executors (RunPacket, ResultPacket, ApprovalPacket, ContextPacket).
pub mod expander;

/// Lexer/tokenizer — winnow-based tokenizer that converts raw `.pgn` text
/// into structured tokens: header, fields, scalars, lists, comments. Enforces
/// input size limits (1 MB max, 100 fields max, 10,000 chars per field).
pub mod lexer;

/// Structured logging — append-only CSV logging for every pipeline stage,
/// with user-value sanitization (printable ASCII only, length-capped,
/// newline-escaped) to prevent log injection and CSV injection.
pub mod logging;

/// Token estimation and cost metrics — estimates token cost of raw text and
/// structured packets using configurable models. Also supports comparing the
/// same handoff expressed as Pidgin vs verbose natural language.
pub mod metrics;

/// Packet parser — winnow-based grammar parser that converts lexed tokens
/// into a typed `PgnPacket` AST with full field validation, directive-specific
/// required-field checks, and human-readable parse errors.
pub mod parser;

/// Registry loader — deserializes YAML configuration files from `.pidgin/`
/// (WorkflowRegistry, ActionRegistry, SafetyRules, ReferenceAliases,
/// PidginRuntimeConfig) with validation of structure and required keys.
pub mod registry;

/// Reference resolver — maps short references (`namespace:id` and bare aliases)
/// to real filesystem paths or identifiers, with canonicalization, host-root
/// containment checks, and symlink-traversal protection.
pub mod resolver;

/// Route planner — recommends an executor for a packet based on workflow
/// configuration and safety results, with fallback chains and logging.
pub mod router;

/// Safety gate — enforces 9 safety rules (SG-1 through SG-9) including
/// contradiction detection, human-approval requirements, private-path
/// protection, workflow validation, mode validation, note isolation,
/// required-ref checking, and critical-risk dual-packet approval.
pub mod safety;

/// Validator — two-stage validation: syntax validation checks structural
/// completeness (required fields present, types correct) and schema
/// validation checks semantic legality (workflow in registry, mode allowed,
/// risk level valid).
pub mod validator;

#[cfg(test)]
mod tests;
