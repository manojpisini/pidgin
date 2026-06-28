//! # Pidgin — A Compact Agent Handoff Protocol & Runtime
//!
//! Pidgin is a small, fast, local-first **protocol and runtime** for compact, structured
//! handoffs between AI agents, between an agent and a human operator, and between an
//! orchestrator and the tools/executors it drives.
//!
//! ```text
//! Take a compact Pidgin packet (9 lines of key=value text)
//! → parse it
//! → validate it against a schema and registries
//! → run it through a safety gate
//! → resolve short references into real paths/IDs
//! → expand it into a fully-specified, executable packet
//! → estimate token cost
//! → recommend a route
//! → log every step
//! ```
//!
//! Pidgin is **not** a model, not an agent framework, not a replacement for
//! MCP/A2A/ACP. It is a narrow waist — a deliberately minimal layer that sits
//! between whatever produces a task and whatever executes it.
//!
//! # Background
//!
//! Every serious 2025–2026 study of multi-agent token cost reaches the same
//! conclusion: the dominant cost and failure surface in multi-agent LLM systems
//! is the communication layer itself, not the reasoning layer.  Replace verbose
//! natural-language handoffs with a small, typed, validated wire format, and
//! you cut both token use and error rate without touching the models.
//!
//! Pidgin formalizes that conclusion into a runtime instead of a one-off convention.
//!
//! # Quick Start
//!
//! ```bash
//! cargo install pidgin-lang
//!
//! # Scaffold a host configuration
//! pgn init
//!
//! # Parse a packet
//! pgn parse examples/basic/generic_task.pgn
//!
//! # Full pipeline
//! pgn run examples/basic/generic_task.pgn
//!
//! # Check (validate + safety + resolve)
//! pgn check examples/basic/generic_task.pgn
//!
//! # Token-cost estimation
//! pgn measure examples/basic/generic_task.pgn
//!
//! # Validate host configuration
//! pgn doctor
//! ```
//!
//! # Architecture
//!
//! The runtime is a linear pipeline — each stage is a pure function that
//! transforms or inspects the packet:
//!
//! ```text
//! Pidgin packet (.pgn text)
//!    │
//!    ▼
//! ┌─────────────┐
//! │   Lexer     │  winnow tokenizer
//! └─────────────┘
//!    │
//!    ▼
//! ┌─────────────┐
//! │   Parser    │  builds typed AST (PgnPacket)
//! └─────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │ Syntax Validator  │  required fields present? types correct?
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │ Schema Validator  │  workflow/mode/risk values legal against registries
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │   Safety Gate     │  contradiction check, deny precedence, human-required
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │ Reference Resolver │ short refs → real paths/IDs with confidence scores
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │  Packet Expander   │  builds fully-specified executable packet (YAML)
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │  Context Planner   │  decides what to retrieve and how
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │  Token Estimator    │  estimates packet + context token cost
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │  Router Planner     │  recommends an executor
//! └──────────────────┘
//!    │
//!    ▼
//! ┌──────────────────┐
//! │  Logger / Metrics    │  every step writes a structured log row
//! └──────────────────┘
//!    │
//!    ▼
//! Expanded packet, ready for: dry-run report | execution handoff | human approval queue
//! ```
//!
//! # Packet Grammar
//!
//! A Pidgin packet is plain text — a header line followed by `key=value` fields:
//!
//! ```text
//! @run task.example
//! wf=generic_review
//! mode=draft
//! in=[primary_subject,source_refs]
//! out=[review_notes]
//! do=[draft,review]
//! deny=[publish,send,delete,secrets]
//! risk=med
//! human=yes
//! ```
//!
//! **Directives:** `@run`, `@result`, `@approval`, `@context`
//!
//! **Reference syntax** (inside lists and the `route` field):
//! - Namespace ref: `namespace:id` — e.g. `file:src/main.rs`, `ep:UNIT012`
//! - Bare alias: resolved through `REFERENCE_ALIASES.yaml`
//!
//! # Safety Gate
//!
//! The safety gate enforces 9 numbered rules (SG-1 through SG-9):
//!
//! | Rule | Description |
//! |------|-------------|
//! | SG-1 | Action in both `do` and `deny` → blocked |
//! | SG-2 | Human-gated action without `human=yes` → blocked |
//! | SG-3 | High/critical risk forces `human=yes`, cannot override |
//! | SG-4 | References resolving to private paths → blocked |
//! | SG-5 | Unknown workflow → blocked |
//! | SG-6 | Invalid mode → blocked |
//! | SG-7 | Free-text `note` field is never parsed for instructions |
//! | SG-8 | Unresolved required inputs → expansion blocked |
//! | SG-9 | Critical risk requires an `@approval` packet |
//!
//! **Safety-first principle:** If the runtime is ever uncertain, it fails closed
//! (blocks, asks for human approval, or refuses to expand) rather than fail open.
//!
//! # Multi-Agent Integration
//!
//! Pidgin is the handoff *format* between agents, not an orchestrator itself.
//! The integration pattern is:
//!
//! ```text
//! Agent A (orchestrator) ──produces .pgn──→ Pidgin (validate→safety→resolve→expand)
//!                                                                                │
//!                                                                           result .pgn
//!                                                                                │
//!                                                                           Pidgin (validate→log)
//!                                                                                │
//! Agent A ◀──────────────────────── reads result ────────────────────────────────┘
//! ```
//!
//! ## Integration with Orchestrator Frameworks
//!
//! - **LangGraph**: A Pidgin node parses/validates the packet before routing to
//!   the next agent. Expanded packets become structured messages in the graph state.
//!
//! - **CrewAI**: Each agent's task output is a `.pgn` packet; Pidgin validates
//!   inter-agent handoffs.
//!
//! - **A2A (Agent2Agent)**: Pidgin's expanded Run Packet is the payload inside
//!   an A2A Task when crossing trust boundaries.
//!
//! - **MCP (Model Context Protocol)**: Pidgin runs as an MCP server, exposing
//!   `parse`, `validate`, `expand` as MCP tools.
//!
//! ## Python SDK
//!
//! A Python SDK (scaffolded in `python/`) wraps the `pgn` binary as a subprocess
//! or via PyO3 bindings, providing typed Pydantic models so orchestrators get
//! Python objects instead of raw CLI output.
//!
//! ## Hooking into the Pipeline
//!
//! Each stage in the pipeline is a public function you can call from your own
//! code:
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
//! You can hook into any individual stage, skip stages, or compose them in
//! custom orders depending on your use case.
//!
//! # Host Configuration
//!
//! Pidgin reads its configuration from `.pidgin/` in the host directory.
//! Use `pgn init` to scaffold the default config:
//!
//! ```bash
//! pgn init
//! ```
//!
//! | File | Purpose |
//! |------|---------|
//! | `WORKFLOW_REGISTRY.yaml` | Workflow definitions with modes, inputs, executors |
//! | `ACTION_REGISTRY.yaml` | Action tiers: `safe`, `controlled`, `human_gated` |
//! | `SAFETY_RULES.yaml` | Default denies, private paths, human approval rules |
//! | `REFERENCE_ALIASES.yaml` | Short-name aliases for frequently-used references |
//! | `PIDGIN_RUNTIME_CONFIG.yaml` | Runtime settings (paths, modes, defaults) |
//!
//! # CLI Reference
//!
//! | Command | Description |
//! |---------|-------------|
//! | `pgn init [--host .] [--force]` | Scaffold default `.pidgin/` config |
//! | `pgn parse <file>` | Parse a packet and print the AST |
//! | `pgn validate <files> --host .` | Validate syntax + schema against registries |
//! | `pgn check <file> --host .` | Parse → validate → safety → resolve (end-to-end) |
//! | `pgn resolve <file> --host .` | Resolve all short references |
//! | `pgn expand <file> --host . [--out file]` | Expand into executable YAML |
//! | `pgn context-plan <file> --host .` | Build a context retrieval plan |
//! | `pgn measure <file>` | Estimate token cost |
//! | `pgn compare --pgn <file> --verbose <file>` | Compare Pidgin vs verbose token cost |
//! | `pgn run <file> --host . [--out file]` | Full pipeline end-to-end |
//! | `pgn doctor --host .` | Check host configuration |
//!
//! # Modules
//!
//! The library is organized into the following modules, each corresponding to a
//! pipeline stage or supporting infrastructure:

/// Typed AST types — `PgnPacket`, `FieldValue`, `PacketDirective`, and related
/// data structures that represent a parsed Pidgin packet in memory.
pub mod ast;

/// Context planner — decides what information to retrieve given a packet's
/// workflow and resolved references, producing a structured retrieval plan.
pub mod context;

/// Error types — `ParseError`, `ValidationError`, `SafetyError`, and other
/// error enums with typed variants and formatted messages.
pub mod errors;

/// Packet expander — takes a parsed, validated, safety-checked, and resolved
/// packet and produces a fully-specified executable YAML packet (RunPacket,
/// ResultPacket, ApprovalPacket, etc.).
pub mod expander;

/// Lexer/tokenizer — winnow-based tokenizer that converts raw `.pgn` text
/// into tokens (headers, fields, scalars, lists, comments).
pub mod lexer;

/// Structured logging — append-only CSV logging for every pipeline stage
/// (parse, validate, safety, resolve, expand, run) with sanitization.
pub mod logging;

/// Token estimation and cost metrics — estimates token cost of raw text and
/// structured packets, and compares Pidgin format against verbose alternatives.
pub mod metrics;

/// Packet parser — winnow-based grammar parser that converts tokenized input
/// into a typed `PgnPacket` AST, with size limits and input validation.
pub mod parser;

/// Registry loader — deserializes YAML configuration files (WorkflowRegistry,
/// ActionRegistry, SafetyRules) from the host's `.pidgin/` directory.
pub mod registry;

/// Reference resolver — resolves short references (`namespace:id`, bare
/// aliases) into real filesystem paths or IDs, with containment checks
/// and symlink traversal protection.
pub mod resolver;

/// Route planner — recommends an executor for a packet based on the
/// workflow's recommended and fallback executors and the safety result.
pub mod router;

/// Safety gate — enforces 9 safety rules (SG-1 through SG-9) including
/// contradiction detection, human-gated actions, private path protection,
/// and post-resolution safety checks.
pub mod safety;

/// Validator — syntax validation (structural completeness) and schema
/// validation (registry-checked legality of values) for parsed packets.
pub mod validator;

#[cfg(test)]
mod tests;
