<div align="center">

```
██████╗ ██╗██████╗  ██████╗ ██╗███╗   ██╗
██╔══██╗██║██╔══██╗██╔════╝ ██║████╗  ██║
██████╔╝██║██║  ██║██║  ███╗██║██╔██╗ ██║
██╔═══╝ ██║██║  ██║██║   ██║██║██║╚██╗██║
██║     ██║██████╔╝╚██████╔╝██║██║ ╚████║
╚═╝     ╚═╝╚═════╝  ╚═════╝ ╚═╝╚═╝  ╚═══╝

              THE AGENT HANDOFF PROTOCOL

█████████████████████████████████████
```

**A compact protocol runtime for agent-to-agent handoffs.** Parse, validate, safety-check, resolve, expand, and log structured messages between agents — with no LLM in the hot path and no network calls in the core.

[![Crates.io](https://img.shields.io/crates/v/pidgin-lang.svg)](https://crates.io/crates/pidgin-lang)
[![Docs.rs](https://img.shields.io/docsrs/pidgin-lang)](https://docs.rs/pidgin-lang)

</div>

---

## Install

```bash
cargo install pidgin-lang
pgn init                    # scaffold .pidgin/ config
pgn --help                  # see all commands
pgn docs                    # full documentation for agents
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
| `serve` | Optional HTTP API/dashboard when built with `--features server` |
| `context-plan` | Build a context retrieval plan |
| `doctor` | Check host configuration health |
| `docs` | Print full protocol documentation as markdown |

Exit codes: `0` success, `1` validation error, `2` safety blocked, `3` unresolved required ref, `4` config error, `5` runtime error.

## Safety Rules (SG-1 through SG-9)

The safety gate enforces nine rules. Every one fails closed:

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

## Multi-Agent Wiring

Each agent system gets its own `.pidgin/` directory. The same binary, different configs per host:

| System | `.pidgin/` Location |
|--------|-------------------|
| LangGraph project | `./.pidgin/` in project root |
| CrewAI crew | `.pidgin/` in each crew's workspace |
| CI pipeline | `.pidgin/` in repo root |
| MCP server | `$PIDGIN_ROOT_DIR` env var |

## Library

```toml
[dependencies]
pidgin-lang = "0.1"
```

```rust
use pidgin_lang::parser::parse_packet;
let packet = parse_packet("@run my.task\nwf=generic_review\nmode=draft")?;
```

Each stage is a public function. Compose your own pipeline, skip stages, or insert custom logic at any point.

## Links

- [Docs.rs](https://docs.rs/pidgin-lang)
- [GitHub](https://github.com/manojpisini/pidgin)
- [crates.io](https://crates.io/crates/pidgin-lang)
