<div align="center">

```
██████╗ ██╗██████╗  ██████╗ ██╗███╗   ██╗
██╔══██╗██║██╔══██╗██╔════╝ ██║████╗  ██║
██████╔╝██║██║  ██║██║  ███╗██║██╔██╗ ██║
██╔═══╝ ██║██║  ██║██║   ██║██║██║╚██╗██║
██║     ██║██████╔╝╚██████╔╝██║██║ ╚████║
╚═╝     ╚═╝╚═════╝  ╚═════╝ ╚═╝╚═╝  ╚═══╝

```
##  THE AGENT HANDOFF PROTOCOL
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
                          ┌─────────────────────────────────────┐
                          │         P I D G I N                 │
                          │  parse → validate → safety → resolve│
                          │  → expand → log                    │
                          └─────────────────────────────────────┘
                               ▲                    │
                    .pgn       │                    │  expanded YAML
                    packet     │                    ▼
                ┌──────────────────┐       ┌──────────────────┐
                │   Orchestrator    │       │  Executor Agent  │
                │  ─────────────── │       │ ──────────────── │
                │  LangGraph node  │       │  Claude          │
                │  CrewAI agent    │       │  Codex           │
                │  A2A Task handler│       │  Custom tool     │
                │  MCP client      │       │  Shell script    │
                │  Custom Python   │       │                  │
                └──────────────────┘       └──────────────────┘
                               ▲                    │
                               │                    │
                               └── result.pgn ──────┘
```

The Python SDK (in `python/`) wraps the binary so orchestrators get typed objects without shelling out manually.

## Wiring Pidgin into Any Multi-Agent System

Each agent system gets its own `.pidgin/` directory with its own workflows, actions, and safety rules. The same binary, different configs per host.

| System | `.pidgin/` Location | Setup |
|--------|--------------------|-------|
| LangGraph project | `./.pidgin/` in project root | Run `pgn init` in the project root. Add a Pidgin validation node between every agent-to-agent edge. The node calls `pgn check` on the source agent's output before passing it to the next agent. Expanded packets become typed state updates. |
| CrewAI crew | `.pidgin/` in each crew's workspace | Each crew gets its own config. Create a custom CrewAI tool that wraps `pgn run`. Every inter-agent task output is validated before the next agent receives it. Blocked handoffs halt the crew and surface the SG rule that fired. |
| CI pipeline | `.pidgin/` in repo root | Run `pgn check` on every `.pgn` file in CI. Validate that packets are well-formed and safe before merging. Use `pgn doctor` to verify the pipeline's own config is healthy. |
| MCP server | `$PIDGIN_ROOT_DIR` env var | Set the env var to point to the config directory. Run Pidgin as an MCP server exposing `validate_packet`, `check_safety`, and `expand_packet` as tools. Any MCP client (Claude Desktop, etc.) can call them. |

### Detailed Setup

#### LangGraph

```python
from langgraph.graph import StateGraph, END
import subprocess

def pidgin_validate_node(state):
    """Between any two agents, validate the handoff."""
    pgn_text = state["agent_output"]  # produced by source agent
    result = subprocess.run(
        ["pgn", "check", "--host", "."],
        input=pgn_text, capture_output=True, text=True
    )
    if result.returncode != 0:
        state["error"] = f"Pidgin blocked: {result.stderr}"
        return state  # route to error handler
    # Safe — expand and pass to next agent
    expanded = subprocess.run(
        ["pgn", "expand", "--host", "."],
        input=pgn_text, capture_output=True, text=True
    )
    state["agent_input"] = expanded.stdout
    return state

# Wire it: agent_a → pidgin_validate → agent_b → pidgin_validate → agent_c
```

#### CrewAI

```python
from crewai import Tool

class PidginValidateTool(Tool):
    name: str = "Pidgin Validate Handoff"
    description: str = "Validate and expand a Pidgin packet before sending to next agent"

    def _run(self, pgn_text: str) -> str:
        result = subprocess.run(
            ["pgn", "check", "--host", "."],
            input=pgn_text, capture_output=True, text=True
        )
        if result.returncode != 0:
            raise ValueError(f"Handoff blocked: {result.stderr}")
        expanded = subprocess.run(
            ["pgn", "expand", "--host", "."],
            input=pgn_text, capture_output=True, text=True
        )
        return expanded.stdout
```

#### CI Pipeline (GitHub Actions)

```yaml
# .github/workflows/pidgin-check.yml
name: Pidgin Check
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo install pidgin-lang
      - run: pgn init --host .
      - run: pgn doctor
      - run: |
          for f in $(find . -name '*.pgn'); do
            pgn check "$f" || exit 1
          done
```

#### MCP Server

```python
# pidgin_mcp_server.py
from mcp.server import Server
import subprocess, json

app = Server("pidgin")

@app.tool("check_safety")
async def check_safety(pgn_text: str) -> str:
    result = subprocess.run(
        ["pgn", "check", "--host", "."],
        input=pgn_text, capture_output=True, text=True
    )
    return json.dumps({
        "pass": result.returncode == 0,
        "output": result.stdout,
        "errors": result.stderr
    })

@app.tool("expand_packet")
async def expand_packet(pgn_text: str) -> str:
    result = subprocess.run(
        ["pgn", "expand", "--host", "."],
        input=pgn_text, capture_output=True, text=True
    )
    return result.stdout if result.returncode == 0 else result.stderr
```

#### A2A (Agent2Agent) Integration

Pidgin + A2A is a natural pair. Pidgin validates inside your trust boundary; A2A carries the expanded task across to external agents:

```
Your System                         External Agent Server
┌──────────────────────────┐        ┌──────────────────────┐
│ Agent A                  │        │ Agent B (external)   │
│   produces .pgn          │        │                      │
│       ↓                  │        │                      │
│   Pidgin check → expand  │        │                      │
│       ↓                  │        │                      │
│   A2A client wraps       │──────→ │  A2A server receives │
│   expanded YAML as       │ A2A    │  Task, routes to     │
│   A2A Task (JSON-RPC)    │ Task   │  Agent B             │
│                          │        │                      │
│   A2A client receives    │←────── │  Agent B produces    │
│   result, logs .pgn      │ result │  result              │
└──────────────────────────┘        └──────────────────────┘
```

The expanded Pidgin packet becomes the payload body of an A2A Task. Pidgin handles validation, safety, and audit — A2A handles discovery, transport, and cross-org boundaries. They solve different problems and work best together.

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
