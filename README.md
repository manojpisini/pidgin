# Pidgin — A Compact Agent Handoff Protocol & Runtime

[![Crates.io](https://img.shields.io/crates/v/pidgin-lang.svg)](https://crates.io/crates/pidgin-lang)

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

**Pidgin** is a small, fast, local-first protocol and runtime for compact, structured handoffs between AI agents, between an agent and a human operator, and between an orchestrator and the tools/executors it drives.

```text
Take a compact Pidgin packet (9 lines of key=value text)
→ parse it
→ validate it against a schema and registries
→ run it through a safety gate
→ resolve short references into real paths/IDs
→ expand it into a fully-specified, executable packet
→ estimate token cost
→ recommend a route
→ log every step
```

Pidgin is **not** a model, not an agent framework, not a replacement for MCP/A2A/ACP. It is a narrow waist — a deliberately minimal layer that sits between whatever produces a task and whatever executes it.

---

## Quick Start

```bash
# Install
cargo install pidgin-lang

# Scaffold a host configuration (creates .pidgin/ with defaults)
pgn init

# Parse a packet
pgn parse examples/basic/generic_task.pgn

# Full pipeline: parse → validate → safety → resolve → expand
pgn run examples/basic/generic_task.pgn

# Check (validate + safety + resolve, with detailed output)
pgn check examples/basic/generic_task.pgn

# Token-cost estimation
pgn measure examples/basic/generic_task.pgn

# Validate host configuration
pgn doctor
```

---

## Packet Grammar

A Pidgin packet is plain text — a header line followed by `key=value` fields:

```text
@run task.example
wf=generic_review
mode=draft
in=[primary_subject,source_refs]
out=[review_notes]
do=[draft,review]
deny=[publish,send,delete,secrets]
risk=med
human=yes
```

| Component | Format | Example |
|-----------|--------|---------|
| Header | `@directive run_id` | `@run task.example` |
| Scalar | `key=value` | `risk=med` |
| List | `key=[val1,val2]` | `in=[a,b,c]` |
| Quoted string | `key="value"` | `note="Draft only"` |
| Comment | `# text` | `# This is a comment` |

**Directives:** `@run`, `@result`, `@approval`, `@context`

**Reference syntax** (inside lists and the `route` field):
- Namespace ref: `namespace:id` — e.g. `file:src/main.rs`, `ep:UNIT012`, `workflow:generic_review`
- Bare alias: resolved through `REFERENCE_ALIASES.yaml`

### Why this grammar?

- Every packet fits in under 15 lines
- Every field is unambiguous to a machine without needing an LLM
- Looks like `.env`/TOML — any developer recognizes it instantly
- One canonical way to write each syntax — no alternatives

---

## CLI Commands

| Command | Description |
|---------|-------------|
| `pgn init [--host .] [--force]` | Scaffold default `.pidgin/` configuration |
| `pgn parse <file>` | Parse a packet and print the AST |
| `pgn validate <files> --host .` | Validate syntax + schema against registries |
| `pgn check <file> --host .` | Parse → validate → safety → resolve (end-to-end check) |
| `pgn resolve <file> --host .` | Resolve all short references |
| `pgn expand <file> --host . [--out file]` | Expand into executable YAML |
| `pgn context-plan <file> --host .` | Build a context retrieval plan |
| `pgn measure <file>` | Estimate token cost |
| `pgn compare --pgn <file> --verbose <file>` | Compare Pidgin vs verbose token cost |
| `pgn run <file> --host . [--out file]` | Full pipeline: parse → validate → safety → resolve → expand |
| `pgn doctor --host .` | Check host configuration |

---

## Host Configuration

Pidgin reads its configuration from `.pidgin/` in the host directory:

| File | Purpose |
|------|---------|
| `WORKFLOW_REGISTRY.yaml` | Workflow definitions with modes, inputs, executors |
| `ACTION_REGISTRY.yaml` | Action tiers: `safe`, `controlled`, `human_gated` |
| `SAFETY_RULES.yaml` | Default denies, private paths, human approval rules |
| `REFERENCE_ALIASES.yaml` | Short-name aliases for frequently-used references |
| `PIDGIN_RUNTIME_CONFIG.yaml` | Runtime settings (paths, modes, defaults) |

Create these files in `.pidgin/` to define your host's capabilities.

---

## Safety Gate

The safety gate enforces 9 numbered rules (SG-1 through SG-9):

- **SG-1**: Action in both `do` and `deny` → blocked
- **SG-2**: Human-gated action without `human=yes` → blocked
- **SG-3**: High/critical risk forces `human=yes`, cannot override
- **SG-4**: References resolving to private paths → blocked
- **SG-5**: Unknown workflow → blocked
- **SG-6**: Invalid mode → blocked
- **SG-7**: Free-text `note` field is never parsed for instructions
- **SG-8**: Unresolved required inputs → expansion blocked
- **SG-9**: Critical risk requires an `@approval` packet

**Safety-first principle:** If the runtime is ever uncertain, it fails closed (blocks, asks for human approval, or refuses to expand) rather than fail open.

---

## Multi-Agent Integration

Pidgin is the handoff *format* between agents, not an orchestrator itself:

```text
Agent A (orchestrator) ──produces .pgn──→ Pidgin (validate→safety→resolve→expand)
                                                                                │
                                                                           result .pgn
                                                                                │
                                                                           Pidgin (validate→log)
                                                                                │
Agent A ◀──────────────────────── reads result ────────────────────────────────┘
```

### Integration Patterns

| Framework | How Pidgin fits |
|-----------|----------------|
| **LangGraph** | Pidgin node parses/validates packets before routing. Expanded packets become structured messages in graph state. |
| **CrewAI** | Agent task outputs are `.pgn` packets; Pidgin validates inter-agent handoffs. |
| **A2A** | Expanded Run Packets drop inside A2A Tasks when crossing trust boundaries. |
| **MCP** | Pidgin runs as an MCP server exposing `parse`, `validate`, `expand` as tools. |
| **Python SDK** | `python/` SDK wraps `pgn` as subprocess or PyO3 calls with typed Pydantic models. |

### Hooking into the Pipeline

Each pipeline stage is a public function you can compose in your own code:

```rust
use pidgin_lang::parser::parse_packet;
use pidgin_lang::safety::check_safety;
use pidgin_lang::expander::expand_to_run_packet;

let packet = parse_packet("@run my.task\nwf=generic_review\nmode=draft")?;
```

---

## Build & Test

### Prerequisites

- Rust stable toolchain (`stable-x86_64-pc-windows-gnu` on Windows)
- On Windows: MSYS2 tools (`ucrt64`) in PATH

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

77 tests (74 unit + 3 property-based fuzz tests), zero warnings.

### Clippy

```bash
cargo clippy --all-targets -- -D warnings
```

### Security audit

```bash
cargo audit
```

The CI pipeline runs build → test → clippy → audit on every push.

---

## Using as a Library

Add `pidgin-lang` to your `Cargo.toml`:

```toml
[dependencies]
pidgin-lang = "0.1"
```

```rust
use pidgin_lang::parser::parse_packet;
use pidgin_lang::safety::check_safety;
use pidgin_lang::expander::expand_to_run_packet;

let packet = parse_packet("@run my.task\nwf=generic_review\nmode=draft")?;
```

All public modules:

- `parser` — parse `.pgn` text into `PgnPacket`
- `lexer` — tokenizer (winnow-based)
- `ast` — typed AST types (`PgnPacket`, `FieldValue`, etc.)
- `validator::syntax` — structural validation
- `validator::schema` — registry-checked value validation
- `safety` — the safety gate (SG-1 through SG-9)
- `resolver` — reference resolution (file, folder, alias, namespace)
- `expander` — packet expansion to executable YAML
- `context` — context retrieval planning
- `metrics` — token cost estimation
- `router` — executor recommendation
- `logging` — structured CSV logging
- `registry` — YAML config deserialization
- `errors` — typed error enum

---

## Project Structure

```
pidgin/
├── Cargo.toml                  # workspace root
├── crates/
│   └── pidgin-lang/            # library + CLI binary
│       ├── src/
│       │   ├── lib.rs          # public module exports
│       │   ├── main.rs         # CLI entrypoint
│       │   ├── ast.rs          # typed packet AST
│       │   ├── parser.rs       # grammar parser
│       │   ├── lexer.rs        # tokenizer
│       │   ├── safety.rs       # safety gate
│       │   ├── resolver.rs     # reference resolver
│       │   ├── expander.rs     # packet expander
│       │   ├── context.rs      # context planner
│       │   ├── metrics.rs      # token estimation
│       │   ├── router.rs       # route planner
│       │   ├── registry.rs     # YAML registry loader
│       │   ├── logging.rs      # structured logging
│       │   └── errors.rs       # error types
│       └── tests/
│           └── proptest_parser.rs
├── .pidgin/                    # host configuration
├── examples/                   # example .pgn packets
├── docs/                       # specification & documentation
├── python/                     # Python SDK (planned)
├── schemas/                    # JSON Schema definitions
└── deny.toml                   # cargo-deny configuration
```

---

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-Apache-2.0) at your option.

---

## Links

- [crates.io: pidgin-lang](https://crates.io/crates/pidgin-lang)
- [Specification](docs/SPEC.md)
- [Security Review](docs/SECURITY_REVIEW.md)
- [GitHub](https://github.com/manojpisini/pidgin)
