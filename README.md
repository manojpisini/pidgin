# Pidgin

**A compact protocol runtime for agent-to-agent handoffs.** Parse, validate, safety-check, resolve, expand, and log structured messages between agents — with no LLM in the hot path and no network calls in the core.

```text
Agent A ── .pgn ──→ Pidgin ── validate → safety → resolve → expand ──→ Agent B
```

[![Crates.io](https://img.shields.io/crates/v/pidgin-lang.svg)](https://crates.io/crates/pidgin-lang)
[![Docs.rs](https://img.shields.io/docsrs/pidgin-lang)](https://docs.rs/pidgin-lang)

---

## Install

```bash
cargo install pidgin-lang
pgn init                    # scaffold .pidgin/ config
pgn --help                  # see all commands
pgn docs                    # full documentation for agents
```

## One-Minute Demo

```bash
# Parse a packet and see the AST
pgn parse examples/basic/generic_task.pgn

# Full pipeline — parse → validate → safety → resolve → expand
pgn run examples/basic/generic_task.pgn

# Safety gate in action: contradiction, missing human, private path
pgn check examples/basic/unsafe_contradiction.pgn
pgn check examples/basic/unsafe_no_human.pgn
pgn check examples/basic/unsafe_private_path.pgn

# Token cost estimation
pgn measure examples/basic/generic_task.pgn
```

## What a Packet Looks Like

```text
@run task.example               # header: directive + run_id
wf=generic_review               # workflow (must be in WORKFLOW_REGISTRY)
mode=draft                      # execution mode
in=[primary_subject,source_refs] # input references
out=[review_notes]              # expected outputs
do=[draft,review]               # requested actions
deny=[publish,send,delete]      # explicit deny list (always wins)
risk=med                        # low | med | high | crit
human=yes                       # human approval required
```

Nine lines. Every field is machine-verified. No ambiguity.

## Why This Exists

Most agent-to-agent handoffs today are verbose natural-language messages dumped into a shared context window. They cost tokens, they fail silently, and nobody audits them. I wanted something that:

- **Validates** every handoff against a schema before anyone acts on it
- **Blocks** dangerous patterns before they happen (contradictory instructions, private file access, missing human approval)
- **Logs** everything to a structured file so you can replay, audit, and learn from every inter-agent message
- **Stays fast** — the whole pipeline runs in single-digit milliseconds on a laptop
- **Stays local** — no network calls, no model calls, no external services

Pidgin is not an agent framework. It is the narrow waist between whatever produces a task and whatever executes it. Think of it as the structured handoff layer that sits underneath MCP, A2A, LangGraph, CrewAI, or any other orchestrator.

## Commands

| Command | What It Does |
|---------|-------------|
| `init` | Scaffold `.pidgin/` directory with default configs |
| `parse` | Parse a `.pgn` file and print the AST |
| `validate` | Syntax + schema validation against registries |
| `check` | Validate → safety gate → resolve (end-to-end check) |
| `resolve` | Expand all short references to real paths/IDs |
| `expand` | Produce fully-specified executable YAML packet |
| `run` | Full pipeline — parse → validate → safety → resolve → expand |
| `measure` | Estimate token cost of a packet |
| `compare` | Compare Pidgin vs verbose token cost |
| `context-plan` | Build a context retrieval plan |
| `doctor` | Check host configuration health |
| `docs` | Print full protocol documentation as markdown |

Exit codes: `0` success, `1` validation error, `2` safety blocked, `3` unresolved required ref, `4` config error, `5` runtime error.

## Safety Rules (SG-1 through SG-9)

The safety gate is the most important part of Pidgin. It enforces nine rules, and every one of them fails closed:

| Rule | What It Catches |
|------|----------------|
| SG-1 | Same action in both `do` and `deny` (contradiction) |
| SG-2 | Human-gated action without `human=yes` |
| SG-3 | High/critical risk with explicit `human=no` |
| SG-4 | Reference resolves to a private path (`.env`, `.ssh/`, etc.) |
| SG-5 | Unknown workflow identifier |
| SG-6 | Mode not in workflow's allowed modes |
| SG-7 | `note` field is never parsed for instructions (injection surface closed) |
| SG-8 | Required input reference failed to resolve |
| SG-9 | Critical risk package without an approval packet |

## Multi-Agent Setup

```
┌──────────┐   .pgn    ┌─────────────────────┐   expanded    ┌──────────┐
│ LangGraph │ ──────→  │  Pidgin Runtime     │ ──────────→  │ Executor │
│ CrewAI    │          │  parse→validate→    │              │ Agent    │
│ A2A/MCP   │ ←──────  │  safety→resolve→    │ ←──────────  │ (Claude, │
│ Custom    │  result  │  expand→log         │  result.pgn  │ Codex…)  │
└──────────┘           └─────────────────────┘              └──────────┘
```

The Python SDK (in `python/`) wraps the binary so orchestrators get typed objects without shelling out manually.

## Host Configuration (`.pidgin/`)

Run `pgn init` to scaffold these files:

| File | Purpose |
|------|---------|
| `PIDGIN_RUNTIME_CONFIG.yaml` | Runtime settings, paths, defaults |
| `WORKFLOW_REGISTRY.yaml` | Workflow definitions with modes and executors |
| `ACTION_REGISTRY.yaml` | Action tiers: safe / controlled / human-gated |
| `SAFETY_RULES.yaml` | Deny list, private paths, human approval rules |
| `REFERENCE_ALIASES.yaml` | Short-name aliases for file/namespace refs |

## Using as a Library

```toml
[dependencies]
pidgin-lang = "0.1"
```

```rust
use pidgin_lang::parser::parse_packet;
use pidgin_lang::safety::check_safety;
use pidgin_lang::expander::expand_to_run_packet;

let packet = parse_packet("@run my.task\nwf=generic_review\nmode=draft")
    .expect("valid packet");
```

Each stage is a public function. You can compose your own pipeline, skip stages, or insert custom logic at any point.

## Build from Source

```bash
cargo build --release
cargo test                  # 78 tests (74 unit + 3 proptest + 1 doctest)
cargo clippy -- -D warnings # zero warnings enforced in CI
cargo audit                 # zero advisories
```

Windows note: requires `stable-x86_64-pc-windows-gnu` toolchain and MSYS2 `ucrt64` in PATH.

## License

MIT or Apache 2.0 — your choice. See [LICENSE](LICENSE).
