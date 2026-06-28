# Pidgin — A Compact Agent Handoff Protocol & Runtime

**Full implementation plan: research grounding, protocol specification, and from-zero build sequence**

**Status:** Independent, vault-agnostic project specification (v2 — supersedes the original `hsir-runtime` draft)
**Relationship to Heap & Stack:** None required. This project is deliberately decoupled from any single vault, brand, or content-operations system so it can be open-sourced, reused, and adopted independently. Heap & Stack (or any other system) can *mount* Pidgin the same way any other host would.

---

## 0. Naming

### 0.1 Why HSIR had to go

The original name — **HSIR**, "Heap Stack Intermediate Representation" — ties a general-purpose protocol runtime to one specific branded system (Heap & Stack). That's a problem for three concrete reasons:

1. **Adoption ceiling.** Nobody outside the Heap & Stack project will adopt a tool whose name signals "this belongs to someone else's content operating system."
2. **Conceptual mismatch.** The runtime described in this document has nothing to do with heaps or stacks as data structures, and nothing to do with "intermediate representation" in the compiler sense (it doesn't sit between two compilation stages — it sits between *agents*). The acronym was a vault-specific in-joke wearing a protocol's clothes.
3. **Trademark and search hygiene.** A name should be checked for collisions before a single line of code is written. "HSIR" was never checked; it also reads as an unpronounceable acronym, which hurts community adoption (compare the one-syllable, sayable names that succeeded: `nom`, `pest`, `tokio`, `serde`, `clap`).

### 0.2 Naming criteria

Following naming guidance distilled from how successful infrastructure projects in this exact space are named — A2A ("Agent2Agent," descriptive-functional), MCP ("Model Context Protocol," descriptive-functional), ACP, ANP, Agora (named after the Greek public square — abstract/metaphorical) — and from Rust ecosystem convention (short, lowercase, no `-rs`/`rust-` decoration, preferably one word)A2A defines Agent Cards for capability advertisement, Tasks for the structure of exchanged work, and a transport binding over HTTP, SSE, and JSON-RPC 2.0, and is maintained as an open-source project, the new name should be:

- **Generic** — not tied to any one vault, brand, or vertical.
- **Abstract / metaphorical** — describes the *shape* of what the system does, not a literal acronym of its internals.
- **Pronounceable and short** — one or two syllables ideally, typeable as a CLI binary name.
- **Unclaimed** — checked against GitHub, crates.io, npm, and PyPI before being treated as final.

### 0.3 Recommended name: **Pidgin**

**Pidgin** (working binary name: `pgn`, package name: `pidgin`) — named after the linguistic phenomenon of a *pidgin*: a deliberately minimal, structurally simplified contact language that two or more parties who don't share a full language improvise so they can get something done together, reliably, with no ambiguity. It is checked as unclaimed across GitHub, crates.io, and npm as of this document's research pass (June 2026).

**Why this fits the system being built, literally, not decoratively:**

- A real pidgin strips grammar down to the load-bearing minimum — fixed word order, no inflection, no idiom — precisely so meaning survives between parties with no shared native tongue. That is exactly what the packet grammar in Part 4 does: it strips natural-language handoffs down to a fixed, terse, unambiguous key=value form so a human, an orchestrator, and an executor — none of which "speak" each other's native format — can still hand off a task without loss.
- A pidgin is never anyone's native or final language; it's an interface language that sits *between* two systems. That maps exactly onto this project's architectural role (Part 1.3): Pidgin is not an agent, not a framework, not a destination — it's the thing both sides briefly speak so the handoff goes through cleanly.
- The word is short, pronounceable, immediately graspable, and carries a small amount of built-in personality without leaning on a private joke or an acronym that needs explaining.
- It is generic enough to describe a protocol used by any agent framework, in any vault, for any domain — exactly the portability goal stated in the original plan.

**Two backup candidates**, in case `pidgin` turns out to collide with something during the person's own final registry check (search engines and registries change weekly, and no single search pass is exhaustive):

| Name | Rationale | Note |
|---|---|---|
| **Quoin** *(rhymes with "coin")* | A quoin is the wedge-shaped stone at the corner of a masonry arch that locks the whole structure together under load — a precise metaphor for a strict, load-bearing protocol layer between agents. Also an old typesetting term (a quoin locks type into a press frame), which doubles the metaphor for a system that "locks" structured packets into place. | Shorter, but slightly less searchable due to homophones. |
| **Narrows** | A geographic narrows is the load-bearing, traffic-control point between two larger bodies — exactly the "sits between, controls flow, doesn't replace either side" role this runtime plays. | Slightly more literal than `pidgin`; reads well as a sentence ("messages pass through the Narrows"). |

This document proceeds using **Pidgin**. Every config key, binary name, and file path below assumes this name; a single find-and-replace converts the whole document to either backup if preferred.

> **Action item for the person before publishing:** run a final, manual check of `pidgin` on GitHub, crates.io, npmjs.com, and PyPI at the time of registration — name availability shifts continuously and no automated research pass (including this one) can guarantee real-time uniqueness.

---

## 1. Executive Summary

Pidgin is a small, fast, local-first **protocol and runtime** for compact, structured handoffs between AI agents, between an agent and a human operator, and between an orchestrator and the tools/executors it drives.

It does one job, and does it well:

```text
Take a compact Pidgin packet (a handful of lines of key=value text)
→ parse it
→ validate it against a schema and a set of registries
→ run it through a safety gate (block dangerous or contradictory instructions)
→ resolve any short references it contains into real paths/IDs
→ expand it into a fully-specified, executable packet
→ build a context plan (what should be retrieved, and how much it will cost)
→ estimate token cost
→ recommend a route (which executor/agent should handle it)
→ log every step
→ optionally hand the expanded packet to an executor and validate what comes back
```

It is **not** a new agent framework, not a replacement for an LLM orchestration library (LangGraph, CrewAI, AutoGen, OpenAI's Agents SDK), not a vector database, not a model gateway, and not MCP or A2A. It is a **narrow waist** — a deliberately small layer that sits between whatever produces a task (a human, an orchestrator, another agent) and whatever executes it (a coding agent, a retrieval pipeline, a publishing action), the same architectural role that the OSI model's IP layer plays between applications and physical networks: nobody runs their whole stack at that layer, but everything passes through it.

```text
Human / Orchestrator / Upstream agent
        ↓
    Pidgin
   (parse → validate → safety-gate → resolve → expand → context-plan → route → log)
        ↓
Executor (coding agent, retrieval pipeline, publishing action, another LLM)
        ↓
Validated Result Packet
        ↓
Logs + (optional) memory/learning candidate
```

### 1.1 Why this needs to exist (the research case, not just the architecture case)

This is not a hypothetical problem. Multiple independent lines of 2025–2026 research converge on the same finding: **the dominant cost and failure surface in multi-agent LLM systems is the communication layer itself, not the reasoning layer.**

- A 2026 survey of token economics in LLM agent systems identifies "message-level communication compression" as a first-class optimization target distinct from model choice or graph topology, citing structural reformatting (typed fields, YAML-style role specs instead of prose) as a zero-training-cost way to cut both token use and error rate: CodeAgents replaces natural-language system prompts and plans with YAML role specifications and Python-style pseudocode, in which typed variables, control structures, and inline assertions encode planning and tool invocation in a more compact form, and this purely structural reformatting yields consistent improvements in both accuracy and token usage, indicating that a non-trivial fraction of redundancy in untrained agent communication originates from the rhetorical and syntactic overhead of natural prose.
- A 2026 paper on identity-aware multi-agent protocols states this even more directly as a design principle: communication overhead is a first-order cost, token consumption directly determines latency and monetary cost, and negotiating compact payload formats such as structured semantic frames instead of verbose natural language yields significant efficiency gains. The same paper notes that governance — provenance tracking, trust boundaries, policy enforcement — cannot be reliably retrofitted onto stateless, opaque protocols, which is precisely the argument for building the safety gate and logging into the runtime's core rather than bolting it on later.
- Research on token-efficient multi-agent topologies shows that *learned or structural* communication compression produces large, measurable savings without hurting accuracy — for example, one routing-policy approach uses 112K tokens on MMLU-Pro, only 15% above a single-agent baseline and four to twenty-four times lower than other multi-agent methods, and a dynamic agent-pruning method achieves an average reduction of 21.6% in prompt token consumption and 18.4% in completion token consumption, along with a performance improvement, compared to state-of-the-art baselines. Pidgin is the structural-compression half of this picture: a fixed, terse grammar instead of free natural language, applied at the handoff boundary rather than inside a single agent's reasoning.

In short: every serious 2025–2026 study of multi-agent token cost reaches the same architectural conclusion this project already reached independently — replace verbose natural-language handoffs with a small, typed, validated wire format. Pidgin formalizes that conclusion into a runtime instead of a one-off convention.

### 1.2 Where Pidgin sits relative to the real protocol landscape

This matters because building Pidgin in ignorance of A2A, MCP, ACP, and ANP would mean re-deriving lessons those projects already paid for in production. Pidgin is explicitly **not** a competitor to any of them — it is a *local, narrow, single-host convention* that can sit underneath all of them.

| Protocol | Governing body | Scope | Transport | Relationship to Pidgin |
|---|---|---|---|---|
| **MCP** (Model Context Protocol) | Originally Anthropic; donated to the Agentic AI Foundation (AAIF) under the Linux Foundation in December 2025 | Connects one AI application to external tools, data sources, and prompts ("agent ↔ tool") | JSON-RPC 2.0 over stdio or Streamable HTTP | Pidgin can run *as* an MCP server (exposing `parse`, `validate`, `expand` as MCP tools) and can also consume MCP servers as resolver backends. MCP provides a standardized interface for reading files, executing functions, and handling contextual prompts, and was created at Anthropic to address the challenge of information silos and legacy systems. |
| **A2A** (Agent2Agent) | Originally Google; under the Linux Foundation since June 2025; reached v1.0 in 2026 | Connects independent agents across organizations/vendors for task delegation ("agent ↔ agent") | HTTP, Server-Sent Events, JSON-RPC 2.0; capability discovery via Agent Cards | A2A defines a way for agents to advertise what they can do through Agent Cards, a structure for the work they exchange called Tasks, and a transport for sending that work over the wire using HTTP, SSE, and JSON-RPC 2.0. Pidgin's expanded Run Packet is a natural payload to drop inside an A2A Task when crossing a trust boundary; Pidgin itself stays local-first and does not attempt cross-organization discovery. |
| **ACP** (Agent Communication Protocol, IBM) | IBM Research, under the Linux Foundation | REST-style multi-agent message exchange with MIME-typed multimodal parts | HTTP/REST, SSE for streaming | ACP-IBM is designed for frictionless integration, treating every agent as an easily accessible REST-style web service, with a message schema centered on roles and multimodal Parts that allows agents to exchange text, images, audio, or artifacts within a unified envelope. Its "Agent Manifest" offline-discovery model is the closest existing analogue to Pidgin's local workflow/action registries. |
| **ANP** (Agent Network Protocol) | Open community effort | Decentralized, internet-scale agent discovery and identity | Decentralized identifiers (DIDs), semantic descriptors | ANP supports secure agent discovery, interaction, and coordination across open, decentralized environments such as agent marketplaces or federated networks, positioned as the top layer in the interoperability stack, building on MCP, ACP, and A2A to enable global-scale agent ecosystems. Out of scope for Pidgin entirely — Pidgin never needs to discover an agent it doesn't already trust and locally configure. |

The layering consensus across the literature is explicit: MCP acts as the foundational layer focused on standardized access to tools and contextual data, ACP complements this by introducing robust message exchange infrastructure, A2A builds on both by enabling dynamic task-centric peer interaction, and ANP extends interoperability to the open internet through decentralized identity and platform-agnostic semantics. Pidgin deliberately occupies a layer *below all four*: it is the local convention an individual host uses to decide what to say before it says it over any of these wires. This is consistent with how IBM frames ACP's own origin relative to MCP — ACP's development was initially intertwined with Anthropic's Model Context Protocol, a standard focused on connecting agents to tools, before ACP diverged to specifically address robust agent-to-agent interactions — the lesson being that compact, well-scoped layers tend to outlive any single framework built on top of them.

### 1.3 What Pidgin explicitly refuses to become

```text
NOT a model-hosting service
NOT an agent reasoning engine
NOT a prompt-generation system
NOT a vector database or memory store
NOT a replacement for LangGraph / CrewAI / AutoGen / any orchestrator
NOT a replacement for MCP, A2A, ACP, or ANP
NOT a publishing system
NOT a system that calls external network services by default
```

Pidgin is infrastructure. It should be boring, fast, deterministic, and almost invisible when it's working correctly — the same design goal MCP itself states for its own scope: MCP takes some inspiration from the Language Server Protocol, which standardizes how to add support for programming languages across a whole ecosystem of development tools, and in a similar way MCP standardizes how to integrate additional context and tools into the ecosystem of AI applications. Pidgin aims at the same kind of narrow, durable standardization, but one level further down — at the level of "what does a single, well-formed instruction from one part of a system to another even look like?"
## 2. Design Philosophy

Pidgin should be:

```text
small          — a single binary should do the whole core pipeline
fast           — sub-10ms for the common case, no LLM in the hot path
local-first    — works fully offline; no required network calls
host-agnostic  — no built-in assumption about any specific vault, framework, or vendor
schema-driven  — every packet type has an explicit, versioned schema
human-auditable — every packet is plain text a person can read without tooling
safe by default — the default posture denies risky actions until a human approves them
easy to test    — pure functions wherever possible; the safety gate must be exhaustively tested
easy to extend  — new workflows/actions/outputs are config, not code changes
```

It should avoid:

```text
model hosting or invoking an LLM directly inside the core pipeline
agent reasoning or planning logic
prompt generation as core logic
mandatory external dependencies (network, database, cloud service)
large always-on background services by default
secret or credential handling of any kind
automatic publishing, sending, or deletion
silent schema drift (every breaking change bumps the spec version)
```

### 2.1 The single most important design decision

Every other decision in this document follows from one rule, stated explicitly so it can be defended later when something tempts the project to grow past it:

> **A Pidgin packet must always be safe to reject.** If the runtime is ever uncertain whether a packet is valid, safe, or fully resolved, it must fail closed (block, ask for human approval, or refuse to expand) rather than fail open (guess, auto-correct, or proceed with partial information).

This single rule is what keeps the system "boring" under pressure, and it is the same principle MCP's own specification states about itself: the Model Context Protocol enables powerful capabilities through arbitrary data access and code execution paths, and with this power comes important security and trust considerations that all implementors must carefully address — including that tool descriptions and behavior annotations should be considered untrusted unless obtained from a trusted server. Pidgin applies the identical posture to its own packets: a packet's *claims* about itself (its declared risk level, its declared safety) are never trusted blindly; the safety gate re-derives them independently from the registries.

---

## 3. Language and Tooling Stack — With Justification

### 3.1 Final stack

```text
Core runtime:        Rust
CLI:                 Rust (clap)
Grammar/parser:      Rust (winnow — see 3.3 for why)
Config/schema:       Rust (serde + serde_yaml + serde_json + jsonschema)
Safety gate:         Rust
Reference resolver:  Rust
Packet expander:     Rust
Logging/metrics:     Rust
File watching:       Rust (notify)
Hashing:             Rust (blake3)

Python SDK:          Python 3.11+ (pydantic v2, typer)
Orchestrator nodes:  Python (framework-agnostic adapter functions; see Part 14)
Structured-output
  validation:        Python (pydantic v2; optional Instructor/Outlines adapters)

Optional later:      TypeScript SDK, WASM build, local HTTP daemon
```

### 3.2 Why a Rust core (not Python-only)

This is not an aesthetic choice. Four independent, falsifiable reasons:

1. **Hot-path performance.** The runtime's job is to sit on every single inter-agent handoff. If it adds 200ms of interpreter startup and import time per call, it becomes the bottleneck it was built to remove. A compiled, single-binary CLI avoids interpreter startup cost entirely.
2. **Single-binary distribution.** A Rust binary built with `cargo build --release` produces one static-ish executable per platform with no runtime dependency tree to install. This matters specifically for the "small local model can build this" goal stated in this project's requirements: a small model does not need to reason about virtual environments, `pip` resolution conflicts, or Python version mismatches if the *core* is a single binary it compiles once and never touches again.
3. **Parser correctness under malformed input.** The packet grammar (Part 4) is deliberately terse, which means it is also easy to get subtly wrong by hand. A typed, compiled parser with exhaustive enum matching catches a missing match arm *at compile time* — a class of bug that a Python dict-based parser would only catch at runtime, if ever.
4. **Memory safety without a garbage collector.** The resolver and safety gate touch the filesystem and process untrusted text. Rust's ownership model removes a whole category of memory-safety bugs that would otherwise need to be defended against by hand in a systems-level scanner.

### 3.3 Why `winnow` for the parser, specifically, not `pest` and not a hand-rolled parser

This is the kind of decision a small local model executing this plan needs spelled out, because "use a parser" is not specific enough to act on.

- **`pest` (PEG/grammar-file based):** generates a `Rule` enum from a separate `.pest` grammar file. Comparative analysis of PEG-vs-combinator approaches notes a structural weakness directly relevant here: when a grammar rule is defined, the rule's structure is known with certainty by the grammar author, yet the generated code still requires manually extracting fields from untyped `Pair` objects, and a change to the grammar can silently break extraction code without a compile-time error, because the grammar and the Rust code that consumes it are two separate, only loosely-linked artifacts. For a packet grammar that **will** evolve across versions (Part 4.6), this loose coupling is a liability.
- **`nom` (parser combinators, macro-based legacy and modern function-based):** widely regarded as faster than most parser-generators, including `pest`, and good for both binary and textual streaming formats, but its documentation has historically been difficult to learn from and its combinator selection narrower than ideal.
- **`winnow`** is a fork of `nom` created specifically to fix `nom`'s biggest weaknesses: winnow started as a fork of nom because its toolbox model of composable parsers worked better than the framework model used by other parser libraries, with explicit goals to improve developer experience and eliminate a performance cliff that existed in certain `nom` usage patterns. Winnow is explicitly positioned as "a parser toolbox, making it easier to get up and running with your parser without getting in the way of hand-writing the trickier parts" and is description as aiming to be a "do anything parser" the way people treat a general-purpose regex engine. It is also the parser actually used in production by `toml_edit`, a comparably terse, line-oriented, key=value-flavored format — the closest existing real-world analogue to the Pidgin grammar.
- **Hand-rolled lexer + recursive-descent parser:** rejected for v1 because the grammar (Part 4) is simple enough that a combinator library removes boilerplate without removing control, and because winnow stays introspectable and customizable at any level, in contrast to a "batteries included" framework (such as `chumsky`) that requires routing everything through its own system rather than composing small functions.

**Decision:** `winnow` is the parser-combinator library for `pidgin-core`. If a future contributor strongly prefers `pest`'s grammar-file readability, the grammar in Part 4 is simple enough that porting later is a contained, low-risk task — but v1 ships with `winnow`.

### 3.4 Why Python for the SDK layer (not Rust bindings only)

Python is not optional, because the ecosystem Pidgin plugs into is overwhelmingly Python-based:

- Most agent orchestration frameworks (LangGraph, CrewAI, AutoGen, the OpenAI Agents SDK) are Python-first or Python-only in their primary SDK.
- Structured-output validation tooling (Pydantic v2, Instructor, Outlines) is Python-native.
- The realistic adoption path for *any* new protocol runtime is "pip install it, import a client, get a typed object back" — not "shell out to a binary and hand-parse stdout."

The Python SDK is a thin wrapper: it either (a) shells out to the compiled `pgn` binary and parses its JSON output, or (b) in a later phase, calls into the Rust core via PyO3 bindings for in-process use without process-spawn overhead. Phase 1 of the roadmap (Part 8) deliberately starts with option (a) — the subprocess approach — because it requires zero unsafe FFI code and lets the Python SDK ship the same week the CLI does.

### 3.5 Why not a two-stage "Python prototype, then Rust rewrite" plan

The original draft of this project proposed building a Python prototype first and rewriting the core in Rust once the spec stabilized. This document recommends **against** that path, for a reason specific to the stated goal of this rewrite: a plan executable by a small local model benefits enormously from **not having to make the same architectural decision twice.** A prototype-then-rewrite plan means the model has to design the grammar, validator, and safety gate logic *twice*, in two different languages, and then reconcile any drift between them. Building directly in Rust, with the Python SDK as a thin client from day one, means there is exactly one source of truth for the grammar and the safety rules at every point in the build. The roadmap in Part 8 is sequenced for Rust-first construction specifically because of this.

### 3.6 Full crate and package list

```toml
# Cargo.toml workspace members: pidgin-core, pidgin-cli, pidgin-daemon (later)

[dependencies]
winnow      = "0.6"     # packet grammar parser (see 3.3)
serde       = { version = "1", features = ["derive"] }
serde_yaml  = "0.9"     # YAML packet/config (de)serialization
serde_json  = "1"       # JSON packet (de)serialization
jsonschema  = "0.18"    # validating expanded packets against JSON Schema
clap        = { version = "4", features = ["derive"] }   # CLI argument parsing
thiserror   = "1"       # typed, ergonomic error enums
anyhow      = "1"       # error propagation at the CLI boundary
walkdir     = "2"       # vault directory scanning
ignore      = "0.4"     # .gitignore-aware file walking (private path exclusion)
rayon       = "1"       # parallel validation of many packets at once
notify      = "6"       # filesystem watch mode
blake3      = "1"       # fast file hashing for the resolver cache
csv         = "1"       # CSV log writers
chrono      = "0.4"     # timestamps in logs

[dev-dependencies]
insta       = "1"       # snapshot ("golden") testing
criterion   = "0.5"     # benchmarking
proptest    = "1"       # property-based testing of the parser/validator
```

```text
# Python SDK (pyproject.toml)
pydantic   >=2.6      # typed packet models, validation
typer      >=0.12     # CLI ergonomics for the Python wrapper (optional convenience layer)
pyyaml     >=6.0       # YAML round-tripping
rich       >=13.0      # human-readable terminal output for dry-run/debug
pytest     >=8.0       # test runner
ruff       >=0.4       # lint + format in one tool
mypy       >=1.9       # static typing check
```
## 4. The Pidgin Packet Grammar — Formal Specification

This section exists because the original draft showed example packets but never wrote down a grammar precise enough for a parser-combinator implementation to be derived mechanically from it. A small local model cannot reliably infer a grammar from three examples; it needs the grammar written down.

### 4.1 Design goals for the grammar itself

```text
1. Every packet must be expressible in under 15 lines for the common case.
2. Every field must be unambiguous to a machine without needing an LLM to interpret it.
3. The grammar must be a strict subset of "looks like .env / TOML key=value lines"
   so that any developer recognizes the syntax in under 10 seconds.
4. Lists must have one unambiguous textual form: comma-separated inside square brackets.
5. There must be exactly one way to write a comment, one way to write a string,
   and one way to write a list. No optional alternate syntaxes, ever — alternate
   syntaxes are exactly the ambiguity class of bug that motivated the original
   Heap & Stack team to reject Markdown's italics ambiguity in their MX project.
```

### 4.2 Lexical grammar (tokens)

```ebnf
(* Pidgin Packet Grammar v1 — EBNF *)

packet        = header_line , NEWLINE , { field_line , NEWLINE } ;

header_line   = "@" , directive , SPACE , run_id ;
directive     = "run" | "result" | "approval" | "context" ;
run_id        = ident , { "." , ident } ;

field_line    = field_name , "=" , field_value ;
field_name    = ident ;
field_value   = scalar_value | list_value ;

scalar_value  = bare_word | quoted_string | number | boolean ;
list_value    = "[" , [ scalar_value , { "," , scalar_value } ] , "]" ;

bare_word     = ident_char , { ident_char | "-" | ":" } ;
quoted_string = '"' , { any_char_except_quote } , '"' ;
number        = digit , { digit } , [ "." , digit , { digit } ] ;
boolean       = "yes" | "no" | "true" | "false" ;

ident         = ident_char , { ident_char | digit | "_" } ;
ident_char    = letter ;

comment_line  = "#" , { any_char } ;     (* full-line comments only *)

(* Whitespace is insignificant except as a separator; no leading indentation
   is required or permitted before field_line. Trailing whitespace is trimmed. *)
```

### 4.3 Canonical example, annotated field by field

```text
@run task.example                        # header: directive=run, run_id=task.example
wf=generic_draft_and_distribute          # field: workflow identifier (registry-checked)
mode=draft                               # field: execution mode (enum-checked)
in=[ep:UNIT012,source,claims,ledger]     # field: list of short references (resolver input)
out=[draft_a,draft_b,draft_c,approval]   # field: list of expected output identifiers
do=[draft,review]                        # field: list of requested actions (registry-checked)
deny=[publish,send,delete,secrets]       # field: explicit denylist (always wins over `do`)
risk=med                                 # field: enum {low, med, high, crit}
human=yes                                # field: boolean — human approval required before execution
ttl=24h                                  # field: optional time-to-live for the packet itself
note="Draft only, do not publish yet"    # field: optional free-text note (never machine-interpreted)
```

### 4.4 Required vs. optional fields, by directive

| Field | `@run` | `@result` | `@approval` | `@context` | Notes |
|---|---|---|---|---|---|
| `wf` | required | required | optional | required | must exist in `WORKFLOW_REGISTRY` |
| `mode` | required | — | — | optional | must be in the workflow's `allowed_modes` |
| `in` | required | — | optional | required | list of short references, resolved by Part 6 |
| `out` | required | required | — | optional | list of output identifiers |
| `do` | optional | — | — | — | list of requested actions; defaults to workflow's safe actions |
| `deny` | optional | — | — | — | always overrides `do` on conflict (Rule SG-1, Part 5) |
| `risk` | optional | — | required | optional | defaults to the workflow's `risk_default` if omitted |
| `human` | optional | — | required (always `yes`) | — | defaults to `yes` if `risk` is `high` or `crit` |
| `ttl` | optional | — | optional | optional | default `24h` if omitted |
| `route` | optional | — | — | — | explicit executor override; otherwise computed (Part 7) |
| `note` | optional | optional | optional | optional | never parsed for instructions — see Rule SG-7 |
| `status` | — | required | required | — | enum `{ok, partial, failed, blocked}` |
| `produced` | — | required | — | — | list of artifact paths actually written |

### 4.5 Reference syntax (used inside `in=[...]`, `out=[...]`, and `route=`)

```ebnf
short_ref     = ref_namespace , ":" , ref_id
              | bare_alias ;

ref_namespace = "ep" | "rb" | "ledger" | "claim" | "policy"
              | "skill" | "wf" | "file" | "folder" | "dash" | "queue" ;
ref_id        = ident , { "-" | "_" | digit } ;
bare_alias    = ident ;     (* resolved through REFERENCE_ALIASES.yaml — Part 6.4 *)
```

Examples: `ep:EP012`, `ledger:R204`, `file:src/main.rs`, `policy:community_reply`, or a bare alias like `script` that the active alias table expands to a full path.

### 4.6 Versioning rule

Every Pidgin packet implicitly targets the grammar version declared in the runtime's `PIDGIN_RUNTIME_CONFIG.yaml` (`runtime.spec_version`). A packet **may** declare an explicit version with an optional `spec=1.0` field; if present, the runtime rejects the packet outright (error `PGN_E020`, Part 11) rather than attempt cross-version coercion, in line with the project's "fail closed" rule (2.1). Breaking grammar changes always bump the major version; new optional fields bump the minor version only.

### 4.7 What is deliberately *not* in the grammar

```text
no nested objects/maps            (use a referenced file instead — keeps packets flat and skimmable)
no arithmetic or expressions       (the runtime is not a templating engine)
no string interpolation            (a field's value is its value, verbatim)
no multi-line string values        (use `note=` sparingly or reference a file)
no conditional fields ("if X then Y")  (conditionals belong in the orchestrator, not the wire format)
```

This is the same minimalism argument CodeAgents makes for typed pseudocode over natural language: the gains come specifically from removing rhetorical and syntactic overhead, not from adding more expressive power to the wire format. Pidgin's grammar is deliberately less expressive than YAML or JSON on purpose — expressiveness is exactly the thing being traded away for terseness, auditability, and parseability by a tiny grammar.

---

## 5. Core Architecture and Runtime Layers

```text
Pidgin packet (.pgn text)
   │
   ▼
┌─────────────┐
│   Lexer     │  winnow tokenizer — Part 4.2
└─────────────┘
   │
   ▼
┌─────────────┐
│   Parser    │  builds typed AST (PgnPacket struct) — Part 4, Part 9
└─────────────┘
   │
   ▼
┌──────────────────┐
│ Syntax Validator  │  required fields present? types correct? — Part 9.2
└──────────────────┘
   │
   ▼
┌──────────────────┐
│ Schema Validator  │  workflow/mode/risk/output values legal? — Part 9.3, registries (Part 12)
└──────────────────┘
   │
   ▼
┌──────────────────┐
│   Safety Gate     │  contradiction check, deny precedence, human-required check — Part 5 → Part 10
└──────────────────┘
   │  (blocked packets stop here, write to PROTOCOL_ERRORS log, exit code 2)
   ▼
┌──────────────────┐
│ Reference Resolver │ short refs → real paths/IDs, with confidence scores — Part 6
└──────────────────┘
   │
   ▼
┌──────────────────┐
│  Packet Expander   │  builds RUN_PACKET.yaml / CONTEXT_PACKET.yaml / etc. — Part 13
└──────────────────┘
   │
   ▼
┌──────────────────┐
│  Context Planner   │  decides what to retrieve and how (Part 13.3)
└──────────────────┘
   │
   ▼
┌──────────────────┐
│  Token Estimator    │ estimates packet + context token cost — Part 13.4
└──────────────────┘
   │
   ▼
┌──────────────────┐
│  Router Planner     │ recommends an executor — Part 7
└──────────────────┘
   │
   ▼
┌──────────────────┐
│  Logger / Metrics    │ every step above writes a structured log row — Part 15
└──────────────────┘
   │
   ▼
Expanded packet, ready for: dry-run report | execution handoff | human approval queue
```

### 5.1 Layer responsibility table

| Layer | Crate / module | Responsibility | Must never |
|---|---|---|---|
| Lexer/Parser | `pidgin-core::parser` | Text → typed AST | Guess a missing field's value |
| Syntax Validator | `pidgin-core::validator::syntax` | Structural completeness | Apply business-logic rules (that's the schema validator's job) |
| Schema Validator | `pidgin-core::validator::schema` | Registry-checked legality of values | Resolve filesystem paths (that's the resolver's job) |
| Safety Gate | `pidgin-core::safety` | Block unsafe/contradictory packets | Ever be bypassed by a packet's own self-declared `risk` or `human` field without independent verification |
| Resolver | `pidgin-core::resolver` | Short ref → real path/ID, with a confidence score | Silently treat an unresolved required reference as resolved |
| Expander | `pidgin-core::expander` | AST + resolved refs → typed output packet | Touch the network |
| Context Planner | `pidgin-core::context` | Decide *what* to retrieve, not retrieve it | Perform retrieval itself (v1 scope, Part 13.3) |
| Token Estimator | `pidgin-core::metrics::tokens` | Approximate cost | Block execution on its own — it informs, the safety gate decides |
| Router Planner | `pidgin-core::router` | Recommend an executor | Execute the recommendation itself |
| Logger | `pidgin-core::logging` | Append-only structured logs | Lose a log row on crash (writes must be append + flush, never buffered-and-lost) |
| CLI | `pidgin-cli` | Human/script entrypoint | Contain business logic that isn't in `pidgin-core` (CLI is a thin shell) |
| Python SDK | `pidgin_runtime` (PyPI) | Orchestrator-framework integration | Reimplement validation logic independently of the Rust core |
## 6. Repository Structure

```text
pidgin/
├── README.md
├── LICENSE                      (recommend MIT or Apache-2.0 — see Part 16.5)
├── CHANGELOG.md
├── ROADMAP.md
├── CONTRIBUTING.md
├── SECURITY.md
├── CODE_OF_CONDUCT.md
├── justfile                     (task runner — see Part 16.2)
├── .gitignore
├── .editorconfig
├── rust-toolchain.toml           (pins exact Rust version — reproducibility)
├── .github/
│   └── workflows/
│       ├── ci.yml
│       ├── release.yml
│       └── security.yml         (cargo-audit, cargo-deny)
│
├── crates/
│   ├── pidgin-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ast.rs            # typed packet AST
│   │       ├── lexer.rs          # winnow tokenizer
│   │       ├── parser.rs         # winnow grammar (Part 4)
│   │       ├── model.rs          # PgnPacket, RunPacket, ResultPacket, etc.
│   │       ├── validator/
│   │       │   ├── mod.rs
│   │       │   ├── syntax.rs
│   │       │   └── schema.rs
│   │       ├── safety.rs         # the safety gate — Part 10
│   │       ├── resolver.rs       # Part 6 (reference resolution)
│   │       ├── expander.rs       # Part 13
│   │       ├── context.rs        # context planner — Part 13.3
│   │       ├── router.rs         # Part 7
│   │       ├── registry.rs       # loads WORKFLOW/ACTION/OUTPUT registries
│   │       ├── aliases.rs        # REFERENCE_ALIASES resolution
│   │       ├── metrics.rs        # token estimation — Part 13.4
│   │       ├── logging.rs        # CSV/JSONL writers — Part 15
│   │       ├── errors.rs         # error enum — Part 11
│   │       └── tests/
│   │           ├── parser_tests.rs
│   │           ├── validator_tests.rs
│   │           ├── safety_tests.rs
│   │           ├── resolver_tests.rs
│   │           └── expander_tests.rs
│   │
│   ├── pidgin-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── init.rs
│   │       │   ├── parse.rs
│   │       │   ├── validate.rs
│   │       │   ├── check.rs
│   │       │   ├── expand.rs
│   │       │   ├── resolve.rs
│   │       │   ├── context_plan.rs
│   │       │   ├── measure.rs
│   │       │   ├── compare.rs
│   │       │   ├── run.rs        # --dry-run support
│   │       │   ├── watch.rs
│   │       │   └── doctor.rs
│   │       └── output.rs         # shared pretty/json/yaml formatting
│   │
│   └── pidgin-daemon/        # v2 scope — Part 8, Phase 9+
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── server.rs
│           ├── watcher.rs
│           └── api.rs
│
├── python/
│   ├── pyproject.toml
│   ├── README.md
│   └── pidgin_runtime/
│       ├── __init__.py
│       ├── client.py             # subprocess wrapper around `pgn` (Part 14)
│       ├── models.py             # pydantic mirrors of the Rust structs
│       ├── adapters/
│       │   ├── langgraph_nodes.py
│       │   ├── crewai_tools.py
│       │   ├── autogen_adapter.py
│       │   ├── dspy_modules.py
│       │   └── instructor_models.py
│       └── mcp_server.py         # exposes pgn as an MCP server (Part 14.6)
│
├── schemas/                      # JSON Schema, the source of truth for validation
│   ├── PACKET_SCHEMA.json
│   ├── RUN_PACKET_SCHEMA.json
│   ├── RESULT_PACKET_SCHEMA.json
│   ├── CONTEXT_PACKET_SCHEMA.json
│   ├── APPROVAL_PACKET_SCHEMA.json
│   └── MEMORY_CANDIDATE_SCHEMA.json
│
├── configs/                      # the actual registries — Part 12 (generic defaults, no vault-specific values)
│   ├── PIDGIN_RUNTIME_CONFIG.yaml
│   ├── REFERENCE_ALIASES.yaml
│   ├── WORKFLOW_REGISTRY.yaml
│   ├── ACTION_REGISTRY.yaml
│   ├── OUTPUT_REGISTRY.yaml
│   ├── SAFETY_RULES.yaml
│   └── TOKEN_BUDGETS.yaml
│
├── examples/
│   ├── basic/
│   │   ├── generic_task.pgn
│   │   ├── review_workflow.pgn
│   │   ├── reply_draft.pgn
│   │   └── health_check.pgn
│   ├── expanded/
│   │   ├── RUN_PACKET.example.yaml
│   │   ├── CONTEXT_PACKET.example.yaml
│   │   └── RESULT_PACKET.example.yaml
│   └── fixture_workspace/        # a tiny fake "vault" for resolver tests
│
├── tests/
│   ├── parser/
│   ├── validator/
│   ├── safety/
│   ├── resolver/
│   ├── expander/
│   ├── cli/
│   └── fixtures/
│
├── docs/
│   ├── SPEC.md                   # the grammar, normative
│   ├── RUNTIME_ARCHITECTURE.md
│   ├── CLI_REFERENCE.md
│   ├── CONFIG_REFERENCE.md
│   ├── HOOKS.md
│   ├── LANGGRAPH_INTEGRATION.md
│   ├── CREWAI_INTEGRATION.md
│   ├── AUTOGEN_INTEGRATION.md
│   ├── MCP_SERVER.md
│   ├── DSPY_INTEGRATION.md
│   ├── INSTRUCTOR_OUTLINES_INTEGRATION.md
│   ├── PERFORMANCE.md
│   └── HOST_INTEGRATION.md       # generic "how to mount Pidgin into any host system"
│
└── benches/
    ├── parse_bench.rs
    ├── validate_bench.rs
    ├── resolve_bench.rs
    └── expand_bench.rs
```

### 6.1 Generic "host mount" pattern (replaces the Heap & Stack–specific vault mount section)

The original draft hard-coded a single vault's folder numbering (`09_AI_Operating_System/...`) directly into the runtime's expectations. That coupling is removed entirely. Pidgin now defines a **host contract** — any four paths a host system declares in its own `PIDGIN_RUNTIME_CONFIG.yaml` — and the runtime never assumes anything about the rest of the host's folder layout.

```yaml
# Minimal host contract — every host (Heap & Stack, a different vault, a bare repo,
# a CI pipeline) provides these four paths and nothing else is assumed.
host:
  root: "."                                  # the host's working root
  inbox: "handoffs/inbox"                    # where new .pgn packets appear
  outbox: "handoffs/generated"                # where expanded packets are written
  logs: "handoffs/logs"                       # where CSV/JSONL logs are written
  config_dir: ".pidgin"                   # where this host's registries live
```

A host that wants the exact Heap & Stack layout from the original draft simply points these four keys at `09_AI_Operating_System/10_Handoffs/inbox`, `.../generated`, `09_AI_Operating_System/12_Logs`, and `09_AI_Operating_System/09_Tools_Connectors/pidgin`. A host that is a bare Git repo with no vault at all points them at `./.pidgin/inbox`, `./.pidgin/generated`, `./.pidgin/logs`, `./.pidgin/`. The runtime code never branches on which kind of host it is — this is precisely what makes it portable, and precisely what the original spec's section 2.5 ("Portability — the runtime can be used in any vault or repo later") promised but did not yet structurally guarantee.
## 7. The Safety Gate — Full Specification

The safety gate is the single most important subsystem in this project. Every rule below is numbered (`SG-n`) so the test suite (Part 17.3) can be written as one test per rule with full traceability, and so a small local model implementing this in Rust has an unambiguous checklist rather than prose to interpret.

### 7.1 Inputs to the safety gate

```text
the parsed, syntax-valid, schema-valid PgnPacket
the active SAFETY_RULES.yaml
the active ACTION_REGISTRY.yaml
the active WORKFLOW_REGISTRY.yaml entry for this packet's `wf`
```

### 7.2 Numbered rules

| Rule ID | Statement | Rationale |
|---|---|---|
| **SG-1** | If an action appears in both `do` and `deny`, the packet is **blocked**, not silently resolved in either direction. | "Fail closed" (2.1) — an ambiguous self-contradiction must never be guessed away. |
| **SG-2** | Any action in the `human_gated` tier of `ACTION_REGISTRY.yaml` (e.g. `publish`, `send`, `delete`, `credential`) requires `human=yes`. If `human` is absent or `no`, the packet is **blocked**. | Matches MCP's own stance that tool descriptions and behavior should never be trusted blindly — here, a packet's own declared safety is never sufficient on its own. |
| **SG-3** | If `risk` is `high` or `crit`, `human` defaults to `yes` even if the packet omits the field, and an explicit `human=no` on a `high`/`crit` packet is **blocked** (cannot be overridden by the packet itself). | The packet author cannot opt out of human review for high-risk actions merely by omission. |
| **SG-4** | Any reference inside `in=[...]` or `out=[...]` that resolves (Part 8) to a path matching an entry in `SAFETY_RULES.yaml`'s `private_paths` list is **blocked**, regardless of `risk` or `human`. | Private-path protection must be unconditional — it is a boundary, not a risk level. |
| **SG-5** | If `wf` does not exist in `WORKFLOW_REGISTRY.yaml`, the packet is **blocked** ("unknown workflow" — never silently treated as a no-op workflow). | An unrecognized workflow is not a "do nothing" instruction; treating it as such would hide a typo or an attack as a successful no-op. |
| **SG-6** | If `mode` is present but not in the resolved workflow's `allowed_modes`, the packet is **blocked**. | Same fail-closed logic as SG-5, scoped to mode. |
| **SG-7** | The `note` field (and any other free-text field) is **never** parsed for instructions, actions, or references by any layer of the runtime. It is stored and logged verbatim and nothing more. | A free-text field is the most obvious prompt-injection surface in the whole packet; treating it as inert by construction removes an entire bug class rather than trying to sanitize it. This mirrors the general defensive posture recommended for any system that treats external or semi-trusted input as untrusted by default. |
| **SG-8** | If any required input reference (per the workflow's `required_inputs`) fails to resolve (Part 8.5 "missing" status), the packet's expansion is blocked — it can still be parsed and reported on for diagnostics, but it cannot proceed to the expander. | A `RunPacket` built from a partially-resolved input set is a silent failure waiting to happen downstream. |
| **SG-9** | Critical risk (`risk=crit`) packets require an `@approval` packet to exist and be marked `status=ok` before a `@run` packet with the same `run_id` may proceed past the safety gate. | Critical-risk actions get a second, separate, append-only-logged approval artifact — not just a single boolean field on the same packet that's requesting the action. |
| **SG-10** | The safety gate's decision and full reasoning trace (which rule fired, what values triggered it) is always logged to `PROTOCOL_ERRORS.csv` or `PIDGIN_RUNTIME_RUNS.csv`, even when the packet passes. | Auditability is not optional only for failures — a clean pass should be just as traceable as a block, so that "why was this allowed" is always answerable later. |
| **SG-11** | The safety gate never calls an LLM, never makes a network call, and never reads any file outside the host's declared `config_dir` and the specific paths the resolver has already validated. | Keeps the gate itself deterministic, fast, and auditable without a model in the loop — this is the literal meaning of "no LLM calls in core runtime" from the original draft's section 16.1, preserved and made testable here. |

### 7.3 Safety gate output type

```rust
pub struct SafetyResult {
    pub allowed: bool,
    pub blocked: bool,
    pub fired_rules: Vec<SafetyRuleId>,   // e.g. [SG2, SG4] — every rule that fired, not just the first
    pub human_required: bool,
    pub effective_risk: RiskLevel,        // the *computed* risk, which may differ from the packet's declared risk
    pub reasons: Vec<BlockReason>,
}
```

Note `fired_rules` is a `Vec`, not a single value — a packet can trip more than one rule simultaneously (e.g. SG-2 and SG-4 together), and the log (SG-10) must capture all of them, not just the first one encountered, so a human reviewing the block has the complete picture in one place.

### 7.4 Required functions

```text
check_safety(packet: &PgnPacket, resolved: &ResolvedRefs, rules: &SafetyRules) -> SafetyResult
is_human_gated(action: &str, registry: &ActionRegistry) -> bool
computes_effective_risk(packet: &PgnPacket, workflow: &WorkflowEntry) -> RiskLevel
block_reasons(result: &SafetyResult) -> Vec<BlockReason>
```

---

## 8. Reference Resolution

### 8.1 Reference types (restated formally from Part 4.5)

```text
ep:<id>        — episode/unit-of-work identifier (host-defined meaning)
rb:<id>        — research-brief style document identifier
ledger:<id>    — source-of-truth ledger entry identifier
claim:<id>     — a specific factual claim identifier
policy:<id>    — a named policy document
skill:<id>     — a named reusable skill/procedure
wf:<id>        — explicit workflow reference (rare; usually `wf=` is used instead)
file:<path>    — a literal file path relative to host.root
folder:<path>  — a literal folder path relative to host.root
dash:<id>      — a named dashboard/report identifier
queue:<id>     — a named queue identifier
<bare_alias>   — looked up in REFERENCE_ALIASES.yaml; expands to one of the above
```

### 8.2 Resolution algorithm (step by step, for a single reference)

```text
1. Parse the reference into (namespace, id) or (bare_alias).
2. If bare_alias: look it up in REFERENCE_ALIASES.yaml's `common` map.
   - Found → replace with the expanded path/namespace:id, continue at step 3.
   - Not found → mark UNRESOLVED, confidence 0.0, continue to next reference.
3. If namespace is `file` or `folder`: join with host.root, check existence on disk.
   - Exists → RESOLVED, confidence 1.0.
   - Not exists → mark MISSING, confidence 0.0.
4. If namespace is `ep`, `rb`, `ledger`, `claim`, `policy`, `skill`, `dash`, or `queue`:
   delegate to a host-supplied resolver function for that namespace (Part 8.3).
   - The host supplies a small lookup table or function per namespace; Pidgin's
     core does not hard-code knowledge of what an "episode" or "research brief" is —
     this generalizes the original draft's vault-specific `find_episode`,
     `find_research_brief` functions into a single pluggable interface.
5. Record the resolution outcome, the elapsed time, and the method used (direct_path,
   alias_table, host_callback) into the resolver cache (Part 9.6).
```

### 8.3 Host resolver callback interface (generalizing the original `find_episode`-style functions)

```rust
pub trait NamespaceResolver {
    /// Given a bare id (no namespace prefix) for this resolver's namespace,
    /// return a resolved path/identifier, or None if not found.
    fn resolve(&self, id: &str) -> Option<ResolvedRef>;
    /// Which namespace string(s) this resolver claims, e.g. ["ep"], or ["rb", "research_brief"].
    fn namespaces(&self) -> &[&str];
}
```

A host registers one `NamespaceResolver` implementation per custom namespace it cares about, in its own config or a small adapter crate/module. Pidgin's core ships with built-in resolvers only for the host-agnostic namespaces: `file`, `folder`. Every other namespace is opt-in and host-supplied — this is the single biggest structural change from the original draft, which assumed deep, hard-coded knowledge of one specific vault's document types.

### 8.4 Resolved reference type

```rust
pub struct ResolvedRef {
    pub original: String,
    pub namespace: String,
    pub ref_id: String,
    pub resolved_path: Option<PathBuf>,
    pub confidence: f32,          // 1.0 = exact match, lower = fuzzy/alias-chain match
    pub required: bool,           // was this reference in the workflow's required_inputs?
    pub status: ResolutionStatus, // Resolved | Missing | Unresolved
}

pub enum ResolutionStatus { Resolved, Missing, Unresolved }
```

### 8.5 Required functions

```text
resolve_ref(reference: &str, host_root: &Path, aliases: &AliasTable, resolvers: &[Box<dyn NamespaceResolver>]) -> ResolvedRef
resolve_all(packet: &PgnPacket, ctx: &ResolverContext) -> ResolvedRefs
expand_alias(bare_alias: &str, aliases: &AliasTable) -> Option<String>
```

### 8.6 Confidence and caching

Resolution results are cached, keyed by `(namespace, ref_id, file_hash_if_applicable)`, with a blake3 hash used to invalidate the cache only when the underlying file actually changes — not on every run. Cache is stored under `host.config_dir/cache/` and is always `.gitignore`d. This is unchanged in spirit from the original draft's caching section, just generalized to the new host-contract paths from Part 6.1.
## 9. Packet Expansion, Context Planning, Routing, and Token Measurement

### 9.1 Expansion targets

```text
RUN_PACKET.yaml        — fully-specified, executable instruction for an executor
CONTEXT_PACKET.yaml    — what to retrieve, how, and at what token budget
APPROVAL_PACKET.yaml   — a standalone artifact requesting human sign-off
RESULT_PACKET.yaml     — produced by an executor after running; validated on the way back in
MEMORY_CANDIDATE.yaml  — an optional, always-human-gated suggestion for long-term memory promotion
```

### 9.2 Required functions

```text
expand_to_run_packet(packet, resolved_refs, safety_result) -> RunPacket
expand_to_context_plan(packet, resolved_refs) -> ContextPlan
expand_to_approval_request(packet, safety_result) -> ApprovalPacket
validate_result_packet(raw_yaml, schema) -> Result<ResultPacket, ValidationError>
```

### 9.3 Context planning (v1 scope: a plan, not a retrieval engine)

The runtime does **not** perform retrieval itself in v1 — it only decides *what should be retrieved and how*, then hands that plan to whatever retrieval system the host already has (a vector store, a plain grep, a knowledge graph, or nothing at all). This mirrors MCP's own resource model, where MCP enables the sharing of files, state, or memory across agents through a standardized resource primitive, but the *content* of those resources is supplied by whatever server implements them — MCP itself doesn't dictate what kind of storage backend sits behind a resource.

```yaml
context_plan:
  primary_refs:
    - ep:EP012
    - script
    - claims
    - ledger
  retrieval_methods:
    - direct_path        # read the resolved file directly
    - full_text           # no chunking/summarization
    - host_retriever       # delegate to whatever the host's retrieval system is (vector DB, etc.)
  token_budget: 8000
  compression_allowed: true
  fallback_if_over_budget: truncate_lowest_priority_ref
```

### 9.4 Token measurement — and why a character-based estimate is the *correct* v1 choice, not a shortcut

The original draft proposed `estimated_tokens = ceil(char_count / 4)` as a placeholder "until real tokenizers are added." This document keeps that formula for v1, but states explicitly *why* it is defensible rather than apologizing for it: token-counting in a protocol layer exists to **compare relative cost between a Pidgin packet and the verbose natural-language prompt it replaces**, not to predict exact provider billing. The character-based heuristic is accurate enough for that *relative* comparison, and it has zero dependencies, runs in microseconds, and never goes stale when a model provider changes its tokenizer. Real per-model tokenizers (e.g. `tiktoken`-equivalent crates) are an opt-in v2 feature for when *absolute* billing precision is needed (Part 9.6).

This relative-comparison framing is exactly the metric the academic literature on agent communication compression actually reports: studies measuring multi-agent token efficiency report savings as ratios and percentages relative to a verbose baseline — for instance, a learned routing policy using 112K tokens on one benchmark is reported as being only 15% above a single-agent baseline and four-to-twenty-four-times lower than other multi-agent baselines, and a dynamic-pruning method reports a 21.6% reduction in prompt-token consumption relative to its baseline — not as claims about provider-exact token counts. Pidgin's `hsir compare` (now `pgn compare`) command exists specifically to produce this same kind of relative number for a host's own workflows.

```text
estimate_tokens(text: &str) -> usize        // ceil(char_count / 4), v1
measure_packet(packet: &PgnPacket) -> TokenReport
compare_verbose(pgn_text: &str, verbose_text: &str) -> TokenSavingsReport
```

```text
TokenSavingsReport {
    pgn_tokens: usize,
    verbose_tokens: usize,
    savings_ratio: f32,     // 1.0 - (pgn_tokens / verbose_tokens)
    savings_pct_display: String,
}
```

### 9.5 Routing

```yaml
route:
  recommended_executor: claude-code
  reason: long_context_review
  fallback: opencode
  human_required: true
```

```text
route(packet: &PgnPacket, registry: &WorkflowRegistry, safety: &SafetyResult) -> RouteDecision
explain_route(decision: &RouteDecision) -> String   // human-readable, for --explain flag
```

Routing in v1 is registry-driven (each workflow declares a `recommended_executor` and `fallback_executor`), not learned or LLM-driven — keeping with the "no LLM in core runtime" rule (2.1, SG-11). A host that wants smarter, learned routing can layer that on top in its orchestrator, using Pidgin's `RouteDecision` as one input among others; this is directly analogous to how the broader research literature treats routing as a separate, swappable concern from the protocol that carries the message — for example, work on adaptive multi-agent communication explicitly separates the *routing policy* (which agent talks to which) from the *payload format* (what they say), and shows the routing layer alone can be responsible for large token savings independent of payload compression.

### 9.6 Real tokenizer support (v2, opt-in)

```text
v2 feature flag: `--tokenizer <provider>`
Supported (v2): a local-only tokenizer crate appropriate to the target model family
                (e.g. a BPE-based crate for GPT-style models, a SentencePiece-based
                crate for Llama/Mistral-style models).
Never: a network call to a provider's tokenization API as part of the default path —
       this would violate the "no network calls by default" rule (2, SG-11).
```
## 10. CLI Design

Binary name: `pgn`

### 10.1 Full command list

```bash
pgn init [--host .]
pgn parse <file> [--format json|yaml] [--pretty]
pgn validate <file>... [--host .]
pgn check <file> [--host .]
pgn expand <file> [--host .] [--out <path>] [--packet run|approval|context]
pgn resolve <file> [--host .]
pgn context-plan <file> [--host .] [--out <path>]
pgn measure <file>
pgn compare <pgn-file> --verbose <verbose-file>
pgn run <file> [--host .] [--dry-run] [--execute]
pgn watch <folder> [--host .]
pgn doctor [--host .]
pgn daemon [--host .]              # v2
```

### 10.2 Detailed command behavior

#### `pgn init`

```bash
pgn init --host .
```
Creates the host's `config_dir` (default `.pidgin/`) with default registries copied from the runtime's built-in templates, plus the `inbox`, `generated`, and `logs` directories declared in the host contract (Part 6.1). Idempotent — running it twice never overwrites existing customized registries; it only fills in anything missing.

#### `pgn parse`

```bash
pgn parse examples/basic/generic_task.pgn --format json --pretty
```
Outputs the parsed AST. Exit code `0` on success, `1` on a syntax error (with the byte offset and line/column of the failure, since winnow's combinator errors carry span information).

#### `pgn validate`

```bash
pgn validate examples/basic/*.pgn --host .
```
Runs syntax + schema validation only (no safety gate, no resolution). Exit codes: `0` valid, `1` invalid.

#### `pgn check`

```bash
pgn check packet.pgn --host .
```
Runs validate → safety gate → resolve, end to end, but does not expand or write any output files. This is the "tell me everything that's wrong, fast" command.

#### `pgn expand`

```bash
pgn expand packet.pgn --host . --out generated/EP012.dist.RUN_PACKET.yaml --packet run
```
Options: `--format yaml|json`, `--packet run|approval|context`.

#### `pgn resolve`

```bash
pgn resolve packet.pgn --host .
```
Prints every reference and its resolution status/confidence — useful standalone for debugging the resolver without running the rest of the pipeline.

#### `pgn context-plan`

```bash
pgn context-plan packet.pgn --host . --out generated/EP012.dist.CONTEXT_PLAN.yaml
```

#### `pgn measure`

```bash
pgn measure packet.pgn
```
Shows token estimates for the raw packet and (if `--expanded` is passed) the expanded run packet.

#### `pgn compare`

```bash
pgn compare packet.pgn --verbose verbose_prompt.md
```
Shows the `TokenSavingsReport` from Part 9.4.

#### `pgn run --dry-run`

```bash
pgn run packet.pgn --host . --dry-run
```
The single most important command in the whole CLI — runs the *entire* pipeline (parse → validate → safety → resolve → expand → context-plan → measure → route) and reports the outcome, but writes **no** files outside the logs directory and performs **no** external actions, ever, regardless of what the packet requests. `--execute` is a separate, explicit flag required to ever leave dry-run behavior, and `--execute` itself still refuses to perform any `human_gated` action (Part 7.2, SG-2) without a corresponding `APPROVAL_PACKET.yaml` with `status=ok` already on disk.

#### `pgn watch`

```bash
pgn watch handoffs/inbox --host .
```
New `.pgn` file appears → validate → expand → write to outbox → log. Uses the `notify` crate (Part 3.6).

#### `pgn doctor`

```bash
pgn doctor --host .
```
Checks: config files exist and parse cleanly; log directory is writable; schemas are present and valid JSON Schema; no private path patterns accidentally match the `inbox`/`outbox` directories themselves; every example packet in `examples/` still validates against the current registries (catches registry drift early).

### 10.3 Exit codes (global convention across every subcommand)

```text
0 = success
1 = validation error (syntax or schema)
2 = safety blocked (one or more SG-n rules fired)
3 = reference missing/unresolved (and was required)
4 = config error (host contract paths missing, registry malformed)
5 = runtime/internal error (should never happen in normal operation; always a bug report)
```

---

## 11. Configuration Files (Generalized — No Host-Specific Defaults Baked In)

The original draft's registries contained values specific to one content-operations vault (`episode_to_distribution`, `claim_check_review`, folder codes like `09AI`). Those are now **examples that ship in `examples/`**, not defaults that ship in `configs/`. The shipped defaults in `configs/` are intentionally minimal and generic, so that adopting Pidgin for an unrelated domain (a customer-support agent system, a coding-agent pipeline, a research-assistant tool) never requires deleting irrelevant vault-specific entries first.

### 11.1 `PIDGIN_RUNTIME_CONFIG.yaml` (shipped default)

```yaml
runtime:
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
```

### 11.2 `ACTION_REGISTRY.yaml` (shipped default — domain-neutral verbs only)

```yaml
safe:
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
```

### 11.3 `SAFETY_RULES.yaml` (shipped default)

```yaml
default_deny:
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
  action_in_do_and_deny: true        # SG-1
  private_path_referenced: true      # SG-4
  unknown_workflow: true             # SG-5
  invalid_mode: true                 # SG-6
  missing_required_field: true
  dangerous_action_without_human: true   # SG-2
```

### 11.4 `WORKFLOW_REGISTRY.yaml` (shipped default — a deliberately generic starter set, not vault content)

```yaml
workflows:
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
```

A host that wants the original Heap & Stack workflows (`episode_to_distribution`, `claim_check_review`, `weekly_vault_health_check`) adds them as *additional* entries in its own `.pidgin/WORKFLOW_REGISTRY.yaml` — the exact, unmodified YAML from the original draft's section 12.2 still works unchanged as a host-level override; nothing about the runtime needed to know about episodes or claim checks to support it.

### 11.5 `REFERENCE_ALIASES.yaml` (shipped default — empty by design)

```yaml
aliases: {}
common: {}
```

Every host populates this entirely on its own. The runtime ships an empty table on purpose, with extensive comments in the file itself showing the *shape* (using the original draft's `EPS: 03_Production/01_Episodes`-style examples as commented-out illustrations only) rather than any live entries.

---

## 12. Error Codes

```text
PGN_E001 missing required field
PGN_E002 unknown workflow
PGN_E003 invalid execution mode
PGN_E004 invalid risk level
PGN_E005 forbidden action requested (SG-1, SG-2)
PGN_E006 human approval missing (SG-2, SG-3)
PGN_E007 reference not found (resolver: Missing)
PGN_E008 private path referenced (SG-4)
PGN_E009 output schema missing or invalid
PGN_E010 ambiguous route (no recommended_executor resolvable)
PGN_E011 packet too vague (zero usable inputs after resolution)
PGN_E012 contradictory fields (e.g. mode incompatible with risk)
PGN_E013 invalid memory-promotion instruction
PGN_E014 invalid path alias
PGN_E015 unsafe external action requested without approval (SG-9)
PGN_E016 malformed list syntax (grammar, Part 4.2)
PGN_E017 duplicate field in one packet
PGN_E018 invalid agent/executor identifier
PGN_E019 token budget exceeded (informational unless host config makes it blocking)
PGN_E020 unsupported or mismatched spec version (Part 4.6)
```
## 13. The From-Zero Build Plan — Smallest Possible Step Size

### 13.1 How to read this section

This is the part of the document built specifically so that **a small, local LLM with no internet access and modest reasoning ability can execute it end to end**, one numbered step at a time, without needing to infer anything not written down. Each step:

- touches **one file**, or runs **one command**,
- has an explicit **done-condition** the executor can check mechanically (does it compile? does the test pass? does the exit code match?),
- never assumes a step that hasn't happened yet.

This is the same granularity principle that makes structured, typed pseudocode outperform free-form natural-language planning in multi-agent systems: explicit, typed, modular steps reduce ambiguity and let an executing agent (human or model) follow a plan algorithmically instead of having to infer intent. Each phase below ends with an **exit gate** — a single command whose success is the unambiguous signal to move to the next phase. A small model should never move to phase *n+1* without the phase *n* exit gate passing.

### 13.2 Conventions used in every step

```text
[F] = create or edit a file at the given path
[C] = run a shell command
[T] = write or run a test
[G] = phase exit gate (must pass before continuing)
```

### 13.3 Phase 0 — Repository Skeleton and Spec Lock

**Goal:** the repository exists, compiles as an empty workspace, and the grammar/spec documents are committed as the single source of truth before any logic is written.

```text
0.1  [C] mkdir pidgin && cd pidgin && git init
0.2  [F] Create rust-toolchain.toml pinning a specific stable Rust version, e.g.:
       [toolchain]
       channel = "1.78.0"
0.3  [F] Create the top-level Cargo.toml as a workspace:
       [workspace]
       members = ["crates/pidgin-core", "crates/pidgin-cli"]
       resolver = "2"
0.4  [C] mkdir -p crates/pidgin-core/src crates/pidgin-cli/src
0.5  [C] cargo init --lib crates/pidgin-core   (then delete its auto-generated Cargo.toml
         workspace lines if cargo added any, keeping only the [package] section)
0.6  [C] cargo init --bin crates/pidgin-cli      (same cleanup)
0.7  [G] cargo build --workspace   → must succeed with two empty crates.
0.8  [F] Copy this entire document to docs/SPEC.md inside the new repo. This is now
         the normative reference; every subsequent phase quotes section numbers from it.
0.9  [F] Create docs/CLI_REFERENCE.md containing exactly the content of Part 10 above.
0.10 [F] Create docs/CONFIG_REFERENCE.md containing exactly the content of Part 11 above.
0.11 [F] Create examples/basic/generic_task.pgn with exactly:
       @run example.task
       wf=generic_review
       mode=draft
       in=[primary_subject,source_refs]
       out=[review_notes]
       do=[draft,review]
       deny=[publish,send,delete,secrets]
       risk=med
       human=yes
0.12 [G] No build step yet (no logic exists) — gate is simply that 0.1–0.11 exist on disk.
         git add -A && git commit -m "phase 0: skeleton + spec lock"
```

### 13.4 Phase 1 — The Lexer and Parser (Grammar from Part 4)

**Goal:** `pgn parse examples/basic/generic_task.pgn` prints a correct, typed AST. No validation, no safety, no resolution yet — parsing only.

```text
1.1  [C] cd crates/pidgin-core && cargo add winnow serde --features serde/derive
1.2  [F] crates/pidgin-core/src/ast.rs
       Define (verbatim structure, types may use String/Vec<String> for v1 simplicity):

       #[derive(Debug, Clone, PartialEq)]
       pub enum Directive { Run, Result, Approval, Context }

       #[derive(Debug, Clone, PartialEq)]
       pub enum FieldValue { Scalar(String), List(Vec<String>) }

       #[derive(Debug, Clone, PartialEq)]
       pub struct PgnPacket {
           pub directive: Directive,
           pub run_id: String,
           pub fields: std::collections::BTreeMap<String, FieldValue>,
       }
       // BTreeMap (not HashMap) is deliberate: deterministic iteration order makes
       // every later snapshot test (Part 17.2) reproducible byte-for-byte.

1.3  [T] crates/pidgin-core/src/tests/ast_tests.rs
       Write one test: construct a PgnPacket by hand, assert its directive and run_id
       fields are readable. (This test exists only to confirm the struct compiles and
       is usable — it is intentionally trivial.)
1.4  [G] cargo test -p pidgin-core ast_tests   → must pass.

1.5  [F] crates/pidgin-core/src/lexer.rs
       Implement, using winnow combinators, exactly these token-level parsers,
       one function per EBNF production in Part 4.2:
         fn header_line(input: &mut &str) -> ModalResult<(Directive, String)>
         fn field_line(input: &mut &str) -> ModalResult<(String, FieldValue)>
         fn ident(input: &mut &str) -> ModalResult<String>
         fn bare_word(input: &mut &str) -> ModalResult<String>
         fn quoted_string(input: &mut &str) -> ModalResult<String>
         fn list_value(input: &mut &str) -> ModalResult<Vec<String>>
         fn comment_line(input: &mut &str) -> ModalResult<()>
       Each function's body is a direct, mechanical translation of its EBNF rule in
       Part 4.2 — there is deliberately no creative interpretation required here.

1.6  [T] crates/pidgin-core/src/tests/lexer_tests.rs
       One test per function above. Example for header_line:
         assert_eq!(header_line(&mut "@run EP012.dist"), Ok((Directive::Run, "EP012.dist".into())));
       Example for list_value:
         assert_eq!(list_value(&mut "[a,b,c]"), Ok(vec!["a","b","c"]));
       Example for quoted_string:
         assert_eq!(quoted_string(&mut "\"hello world\""), Ok("hello world".into()));

1.7  [G] cargo test -p pidgin-core lexer_tests   → every test passes.

1.8  [F] crates/pidgin-core/src/parser.rs
       Implement:
         pub fn parse_packet(input: &str) -> Result<PgnPacket, ParseError>
       which: splits input into lines, skips comment_line and blank lines, parses the
       first non-comment line with header_line, parses every remaining non-comment
       line with field_line, and assembles a PgnPacket.

1.9  [T] crates/pidgin-core/src/tests/parser_tests.rs
       Test 1: parse_packet on the literal contents of examples/basic/generic_task.pgn
         (read via include_str!) produces a PgnPacket whose directive is Run, whose
         run_id is "example.task", and whose fields map contains "wf" -> Scalar("generic_review").
       Test 2: parse_packet on a string with a missing "@" header line returns Err.
       Test 3: parse_packet on a string with an unterminated quoted string returns Err.
       Test 4: parse_packet on a string with a duplicate field line (e.g. two `wf=` lines)
         returns Err(ParseError::DuplicateField) — this is PGN_E017 (Part 12), caught here.

1.10 [G] cargo test -p pidgin-core parser_tests   → every test passes.

1.11 [F] crates/pidgin-core/src/errors.rs
       Define, using thiserror:
         #[derive(Debug, thiserror::Error)]
         pub enum ParseError {
             #[error("missing @ header line")] MissingHeader,
             #[error("unterminated quoted string at byte {0}")] UnterminatedString(usize),
             #[error("duplicate field: {0}")] DuplicateField(String),
             #[error("malformed list at byte {0}")] MalformedList(usize),
         }
       Wire parser.rs to return these specific variants instead of a generic error.

1.12 [G] cargo test -p pidgin-core   → full crate test suite passes.

1.13 [F] crates/pidgin-cli/Cargo.toml: add dependency on pidgin-core (path = "../pidgin-core")
         and clap (features = ["derive"]).
1.14 [F] crates/pidgin-cli/src/main.rs
       Implement a single subcommand, `parse <file>`, that: reads the file, calls
       pidgin_core::parser::parse_packet, and prints the resulting struct with
       {:#?} (Rust's pretty-debug formatter — this is intentionally NOT JSON yet;
       JSON output formatting is added in Phase 6, Part 13.9, to keep this phase small).

1.15 [G] cargo run -p pidgin-cli -- parse examples/basic/generic_task.pgn
       → must print a populated PgnPacket with no panic. This is the Phase 1 exit gate.
       git add -A && git commit -m "phase 1: lexer + parser, pgn parse works"
```
### 13.5 Phase 2 — Registries, Config Loader, and the Schema Validator

**Goal:** `pgn validate examples/basic/generic_task.pgn` correctly passes for valid packets and correctly fails (with the right `PGN_E0xx` code) for each class of invalid packet.

```text
2.1  [C] cd crates/pidgin-core && cargo add serde_yaml jsonschema
2.2  [F] configs/PIDGIN_RUNTIME_CONFIG.yaml — copy verbatim from Part 11.1.
2.3  [F] configs/ACTION_REGISTRY.yaml — copy verbatim from Part 11.2.
2.4  [F] configs/SAFETY_RULES.yaml — copy verbatim from Part 11.3.
2.5  [F] configs/WORKFLOW_REGISTRY.yaml — copy verbatim from Part 11.4.
2.6  [F] configs/REFERENCE_ALIASES.yaml — copy verbatim from Part 11.5.
2.7  [F] configs/TOKEN_BUDGETS.yaml:
       budgets:
         default_packet_max_tokens: 200
         default_context_max_tokens: 8000

2.8  [F] crates/pidgin-core/src/registry.rs
       Define plain serde-deserializable structs that mirror each YAML file's shape
       1:1 — no transformation logic, just typed deserialization targets:
         pub struct ActionRegistry { pub safe: Vec<String>, pub controlled: Vec<String>, pub human_gated: Vec<String> }
         pub struct WorkflowEntry { pub description: String, pub risk_default: String,
             pub allowed_modes: Vec<String>, pub required_inputs: Vec<String>,
             pub expected_outputs: Vec<String>, pub recommended_executor: String,
             pub fallback_executor: String }
         pub struct WorkflowRegistry { pub workflows: std::collections::BTreeMap<String, WorkflowEntry> }
         pub struct SafetyRules { /* mirrors Part 11.3 exactly, field for field */ }
       Implement:
         pub fn load_action_registry(path: &Path) -> Result<ActionRegistry, ConfigError>
         pub fn load_workflow_registry(path: &Path) -> Result<WorkflowRegistry, ConfigError>
         pub fn load_safety_rules(path: &Path) -> Result<SafetyRules, ConfigError>
       Each is: read_to_string, then serde_yaml::from_str, mapped to ConfigError on failure.

2.9  [T] crates/pidgin-core/src/tests/registry_tests.rs
       Test: load_workflow_registry on configs/WORKFLOW_REGISTRY.yaml succeeds and
       contains a "generic_review" key. Test: loading a deliberately malformed YAML
       string (e.g. truncated) returns Err, never panics.

2.10 [G] cargo test -p pidgin-core registry_tests   → passes.

2.11 [F] crates/pidgin-core/src/validator/syntax.rs
       Implement:
         pub fn validate_syntax(packet: &PgnPacket) -> Vec<ValidationError>
       Checks, per Part 4.4's required-field table, keyed on `packet.directive`:
         - For Directive::Run: "wf", "mode", "in", "out" must be present → else PGN_E001.
         - List-typed fields ("in", "out", "do", "deny") must actually be FieldValue::List,
           not FieldValue::Scalar → else PGN_E016.
       Returns an empty Vec if everything required is present and correctly typed.

2.12 [T] crates/pidgin-core/src/tests/syntax_validator_tests.rs
       Test 1: the example packet (Phase 0) validates with zero errors.
       Test 2: a packet missing `wf=` returns exactly one error, PGN_E001, mentioning "wf".
       Test 3: a packet with `in=not_a_list` (scalar instead of list) returns PGN_E016.

2.13 [G] cargo test -p pidgin-core syntax_validator_tests   → passes.

2.14 [F] crates/pidgin-core/src/validator/schema.rs
       Implement:
         pub fn validate_schema(packet: &PgnPacket, workflows: &WorkflowRegistry) -> Vec<ValidationError>
       Checks:
         - `wf` value exists as a key in workflows.workflows → else PGN_E002.
         - `mode` value (if present) is in that workflow's allowed_modes → else PGN_E003.
         - `risk` value (if present) is one of {low, med, high, crit} → else PGN_E004.
       Note: this function takes an already-loaded WorkflowRegistry as a parameter —
       it never loads config itself. Keeping I/O out of validation logic makes this
       function trivially unit-testable with a hand-built in-memory registry.

2.15 [T] crates/pidgin-core/src/tests/schema_validator_tests.rs
       Test 1: example packet against the real WORKFLOW_REGISTRY.yaml (loaded once in
       a test fixture) validates with zero errors.
       Test 2: a packet with `wf=totally_made_up_workflow` returns PGN_E002.
       Test 3: a packet with `mode=publish_now` (not in generic_review's allowed_modes)
       returns PGN_E003.

2.16 [G] cargo test -p pidgin-core schema_validator_tests   → passes.

2.17 [F] crates/pidgin-cli/src/commands/validate.rs
       Implement the `validate <file>...` subcommand: for each file, parse, then run
       validate_syntax + validate_schema (after loading the registries from the
       resolved host config_dir — for Phase 2, hard-code config_dir to "./configs"
       since the full host-contract config loader is finished in Phase 6). Print a
       one-line PASS/FAIL per file. Exit code 0 if all pass, 1 if any fail (Part 10.3).

2.18 [G] cargo run -p pidgin-cli -- validate examples/basic/generic_task.pgn
       → prints PASS, exits 0. This is the Phase 2 exit gate.
       git add -A && git commit -m "phase 2: registries + syntax/schema validator, pgn validate works"
```

### 13.6 Phase 3 — The Safety Gate (Part 7, Rules SG-1 through SG-11)

**Goal:** every numbered safety rule from Part 7.2 has a corresponding, passing unit test, and `pgn check` correctly blocks unsafe example packets.

```text
3.1  [F] examples/basic/unsafe_contradiction.pgn
       @run unsafe.contradiction
       wf=generic_review
       mode=draft
       in=[primary_subject]
       out=[review_notes]
       do=[publish]
       deny=[publish]
       risk=low
       human=no
       (This packet exists purely to trip SG-1; it should never validate as runnable.)

3.2  [F] examples/basic/unsafe_no_human.pgn
       @run unsafe.no_human
       wf=generic_review
       mode=draft
       in=[primary_subject]
       out=[review_notes]
       do=[publish]
       risk=low
       human=no
       (Trips SG-2: `publish` is human_gated but human=no.)

3.3  [F] examples/basic/unsafe_private_path.pgn
       @run unsafe.private_path
       wf=generic_review
       mode=draft
       in=[file:.env]
       out=[review_notes]
       risk=low
       human=yes
       (Trips SG-4: references a private path.)

3.4  [F] crates/pidgin-core/src/safety.rs
       Define:
         pub enum SafetyRuleId { Sg1, Sg2, Sg3, Sg4, Sg5, Sg6, Sg7, Sg8, Sg9 }
         pub struct SafetyResult {
             pub allowed: bool, pub blocked: bool,
             pub fired_rules: Vec<SafetyRuleId>,
             pub human_required: bool,
             pub effective_risk: String,
         }
       Implement check_safety(packet, workflow_entry, action_registry, safety_rules)
       -> SafetyResult as a sequence of independent rule checks (SG-1 through SG-6 can
       all be checked from the packet + registries alone, before resolution exists —
       SG-4, SG-8, and SG-9 need resolved refs and are layered in fully once Phase 4
       (the resolver) exists; for Phase 3, implement SG-4 against the *unresolved*
       reference string only, by pattern-matching it directly against
       safety_rules.private_paths, which is sufficient for the file:.env case in 3.3
       and gets upgraded to use real resolved paths in Phase 4).
       Each rule check appends to fired_rules if it fires; it does NOT return early —
       per Part 7.3, all fired rules must be collected, not just the first.

3.5  [T] crates/pidgin-core/src/tests/safety_tests.rs
       One test per rule, named exactly after the rule for traceability:
         #[test] fn sg1_do_and_deny_conflict_blocks() { ... uses unsafe_contradiction.pgn ... }
         #[test] fn sg2_human_gated_without_human_blocks() { ... uses unsafe_no_human.pgn ... }
         #[test] fn sg3_high_risk_defaults_human_yes() { ... }
         #[test] fn sg3_explicit_human_no_on_crit_still_blocks() { ... }
         #[test] fn sg4_private_path_blocks_unconditionally() { ... uses unsafe_private_path.pgn ... }
         #[test] fn sg5_unknown_workflow_blocks() { ... }
         #[test] fn sg6_invalid_mode_blocks() { ... }
         #[test] fn sg7_note_field_never_parsed_for_actions() {
             // construct a packet whose note="do=[publish]" and assert that the
             // safety result is IDENTICAL to the same packet with note removed —
             // proves the note field has zero influence on the safety outcome.
         }
         #[test] fn safe_example_packet_passes_with_zero_fired_rules() {
             // the Phase 0 generic_task.pgn example must produce allowed=true,
             // fired_rules=[] — this is the negative-space test that proves the
             // gate doesn't false-positive on ordinary, well-formed packets.
         }

3.6  [G] cargo test -p pidgin-core safety_tests   → every single test passes,
       including the negative-space test. This gate is non-negotiable: do not proceed
       to Phase 4 with any failing safety test, ever, regardless of time pressure.

3.7  [F] crates/pidgin-cli/src/commands/check.rs
       Implement `check <file>`: parse → validate_syntax → validate_schema →
       check_safety, printing a structured report (which rules fired, if any) and
       returning exit code 2 if blocked (Part 10.3), 1 if invalid, 0 if clean.

3.8  [G] Run all three:
         cargo run -p pidgin-cli -- check examples/basic/generic_task.pgn        → exit 0
         cargo run -p pidgin-cli -- check examples/basic/unsafe_contradiction.pgn → exit 2, reports Sg1
         cargo run -p pidgin-cli -- check examples/basic/unsafe_no_human.pgn      → exit 2, reports Sg2
       All three must match exactly. This is the Phase 3 exit gate.
       git add -A && git commit -m "phase 3: safety gate, all SG rules tested, pgn check works"
```
### 13.7 Phase 4 — Reference Resolver (Part 8)

**Goal:** `pgn resolve` correctly resolves `file:` references against a real filesystem fixture, correctly fails closed on missing files, and SG-4/SG-8 now use real resolved paths instead of unresolved string matching.

```text
4.1  [C] cargo add blake3 walkdir -p pidgin-core
4.2  [C] mkdir -p examples/fixture_workspace
4.3  [F] examples/fixture_workspace/primary_subject.md — any placeholder text, e.g.
       "This is a fixture file used by resolver tests."
4.4  [F] examples/fixture_workspace/.env — placeholder secret-looking content, e.g.
       "API_KEY=placeholder-do-not-resolve"
       (This file exists specifically so resolver+safety tests can prove it is never
       successfully resolved into a usable reference.)

4.5  [F] crates/pidgin-core/src/resolver.rs
       Define:
         pub enum ResolutionStatus { Resolved, Missing, Unresolved }
         pub struct ResolvedRef {
             pub original: String, pub namespace: String, pub ref_id: String,
             pub resolved_path: Option<PathBuf>, pub confidence: f32,
             pub required: bool, pub status: ResolutionStatus,
         }
         pub trait NamespaceResolver {
             fn resolve(&self, id: &str) -> Option<ResolvedRef>;
             fn namespaces(&self) -> &[&str];
         }
       Implement a built-in FileResolver (handles "file:" and "folder:" namespaces
       only, per Part 8.3) whose resolve() joins the id with host_root and checks
       Path::exists().
       Implement:
         pub fn resolve_ref(reference: &str, host_root: &Path, aliases: &AliasTable,
             resolvers: &[Box<dyn NamespaceResolver>]) -> ResolvedRef
         pub fn resolve_all(packet: &PgnPacket, ctx: &ResolverContext) -> Vec<ResolvedRef>
       resolve_ref's algorithm is the literal step list in Part 8.2: split on first ":",
       check alias table if no ":" present, dispatch to the matching NamespaceResolver
       by namespace string, else mark Unresolved.

4.6  [T] crates/pidgin-core/src/tests/resolver_tests.rs
       Test 1: resolve_ref("file:primary_subject.md", fixture_root, ...) →
         status=Resolved, confidence=1.0, resolved_path=Some(fixture_root/primary_subject.md).
       Test 2: resolve_ref("file:does_not_exist.md", fixture_root, ...) →
         status=Missing, confidence=0.0.
       Test 3: resolve_ref("totally_unknown_bare_alias", fixture_root, empty_alias_table, ...)
         → status=Unresolved.
       Test 4: resolve_ref("file:.env", fixture_root, ...) → status=Resolved (the
         resolver itself does not block private paths — that is the safety gate's job,
         confirmed in test 5 below — the resolver's only job is "does this path exist").
       Test 5 (integration, spans resolver.rs + safety.rs): resolving "file:.env" and
         then running check_safety on a packet whose `in` includes it must produce
         SG-4 fired, blocked=true. This is the test that upgrades SG-4 from
         string-pattern-matching (Phase 3) to real-path-based matching.

4.7  [G] cargo test -p pidgin-core resolver_tests   → passes, including test 5
       (which exercises the now-updated safety.rs from Phase 3).

4.8  [F] Update crates/pidgin-core/src/safety.rs: change the SG-4 check from
       matching the raw reference string against safety_rules.private_paths to
       matching each ResolvedRef.resolved_path (when Some) against the same patterns,
       using the `ignore`/glob crate's pattern matching. Re-run the Phase 3 safety
       test suite to confirm nothing regressed:
4.9  [G] cargo test -p pidgin-core safety_tests resolver_tests   → all pass.

4.10 [F] crates/pidgin-cli/src/commands/resolve.rs
       Implement `resolve <file>`: parse, build a resolver context rooted at the
       current directory (or --host, defaulting to "."), call resolve_all, print
       a table of reference → status → confidence → resolved_path.

4.11 [G] cargo run -p pidgin-cli -- resolve examples/basic/generic_task.pgn
       → for each of "primary_subject" and "source_refs" (both bare aliases with no
       entry yet in the empty REFERENCE_ALIASES.yaml), correctly reports Unresolved —
       this is the expected, correct outcome for an example packet using placeholder
       alias names with no alias table populated yet. Add one resolvable example
       packet for a fuller demonstration:
4.12 [F] examples/basic/resolvable_task.pgn
       @run resolvable.task
       wf=generic_review
       mode=draft
       in=[file:primary_subject.md]
       out=[review_notes]
       risk=low
       human=yes
4.13 [G] cargo run -p pidgin-cli -- resolve examples/basic/resolvable_task.pgn
       (run with --host pointing at examples/fixture_workspace)
       → reports file:primary_subject.md as Resolved, confidence 1.0.
       This is the Phase 4 exit gate.
       git add -A && git commit -m "phase 4: resolver, pgn resolve works, SG-4 upgraded to real paths"
```

### 13.8 Phase 5 — Packet Expander (Part 9.1–9.2)

**Goal:** `pgn expand` produces a valid `RUN_PACKET.yaml` that itself validates against `schemas/RUN_PACKET_SCHEMA.json`.

```text
5.1  [F] schemas/RUN_PACKET_SCHEMA.json
       A JSON Schema (draft 2020-12) describing the RunPacket shape from Part 13.x
       below — required keys: run_id, workflow, mode, risk_level,
       human_approval_required, inputs (array of resolved-ref objects),
       expected_outputs (array of strings), forbidden_actions (array of strings).
5.2  [C] cargo add jsonschema -p pidgin-core   (already added in 2.1; confirm present)

5.3  [F] crates/pidgin-core/src/expander.rs
       Define:
         #[derive(serde::Serialize)]
         pub struct RunPacket {
             pub run_id: String, pub workflow: String, pub mode: String,
             pub risk_level: String, pub human_approval_required: bool,
             pub inputs: Vec<ResolvedRef>, pub expected_outputs: Vec<String>,
             pub forbidden_actions: Vec<String>,
         }
       Implement:
         pub fn expand_to_run_packet(packet: &PgnPacket, resolved: &[ResolvedRef],
             safety: &SafetyResult) -> RunPacket
       which is a pure, deterministic field-by-field mapping — no logic beyond
       copying values across, by design (Part 5.1 layer table: "Expander... must
       never touch the network", and implicitly: must never re-derive safety
       decisions either — it trusts the already-computed SafetyResult verbatim).
       Per SG-8 (Part 7.2), if any required input's ResolvedRef.status is Missing,
       expand_to_run_packet returns Err(ExpansionError::MissingRequiredInput) instead
       of a RunPacket — expansion is refused, not partially completed.

5.4  [T] crates/pidgin-core/src/tests/expander_tests.rs
       Test 1: expanding the Phase 4 resolvable_task.pgn example produces a RunPacket
       whose run_id == "resolvable.task", workflow == "generic_review",
       human_approval_required == true.
       Test 2: serializing that RunPacket to YAML (serde_yaml::to_string) and then
       validating the resulting YAML (parsed to serde_json::Value) against
       schemas/RUN_PACKET_SCHEMA.json passes with zero schema violations.
       Test 3: a packet with a Missing required input produces
       Err(ExpansionError::MissingRequiredInput), never a partially-filled RunPacket.

5.5  [G] cargo test -p pidgin-core expander_tests   → passes.

5.6  [F] crates/pidgin-cli/src/commands/expand.rs
       Implement `expand <file> --out <path> [--packet run]`: parse → validate →
       safety → resolve → expand_to_run_packet → serde_yaml::to_string → write to
       --out. If safety blocked, refuse to expand at all (print the block reasons,
       exit 2) — expansion of a blocked packet is never attempted, per the pipeline
       order in Part 5 (the safety gate sits before the expander and the expander
       trusts its output, so a blocked packet must never reach this function).

5.7  [G] cargo run -p pidgin-cli -- expand examples/basic/resolvable_task.pgn
         --host examples/fixture_workspace --out /tmp/RUN_PACKET.yaml
       → /tmp/RUN_PACKET.yaml exists, is valid YAML, and validates against
       schemas/RUN_PACKET_SCHEMA.json (confirm with a one-off script or by reusing
       expander_tests' validation helper from the CLI itself via `pgn validate-schema`
       if you choose to add that convenience command — optional, not required for
       the gate). This is the Phase 5 exit gate.
       git add -A && git commit -m "phase 5: expander, pgn expand produces schema-valid RUN_PACKET.yaml"
```
### 13.9 Phase 6 — Host Contract, Remaining Commands, Context Plan, Routing, Token Measurement, Dry-Run

**Goal:** every command in Part 10.1 exists and behaves as specified; `pgn run --dry-run` performs the full pipeline end to end and writes zero external side effects.

```text
6.1  [F] crates/pidgin-core/src/host.rs
       Define:
         pub struct HostConfig {
             pub root: PathBuf, pub inbox: PathBuf, pub outbox: PathBuf,
             pub logs: PathBuf, pub config_dir: PathBuf,
         }
       Implement:
         pub fn load_host_config(host_root: &Path) -> Result<HostConfig, ConfigError>
       which reads <host_root>/.pidgin/PIDGIN_RUNTIME_CONFIG.yaml's `host:`
       block (Part 11.1) and resolves all four paths relative to host_root.
       Implement:
         pub fn default_host_config(host_root: &Path) -> HostConfig
       returning the shipped defaults from Part 11.1 verbatim, used by `pgn init`
       when no config exists yet.

6.2  [F] crates/pidgin-cli/src/commands/init.rs
       Implement `init [--host .]`: if .pidgin/ doesn't exist, create it and copy
       every file from Phase 2's configs/ directory into it (as a starting point the
       host can then edit); create inbox/outbox/logs directories. If .pidgin/
       already exists, only create whatever subset of files/directories is missing
       — never overwrite an existing, possibly-customized registry file.

6.3  [G] cd /tmp && mkdir test_host && cd test_host &&
         cargo run --manifest-path <repo>/Cargo.toml -p pidgin-cli -- init --host .
       → .pidgin/ now exists with all six config files plus inbox/outbox/logs dirs.

6.4  [F] crates/pidgin-core/src/router.rs (Part 9.5)
       Implement:
         pub struct RouteDecision { pub recommended_executor: String, pub reason: String,
             pub fallback: String, pub human_required: bool }
         pub fn route(packet: &PgnPacket, workflow: &WorkflowEntry, safety: &SafetyResult) -> RouteDecision
       Body: recommended_executor = workflow.recommended_executor (unless the packet
       has an explicit `route=` field override, which takes precedence), fallback =
       workflow.fallback_executor, human_required = safety.human_required, reason =
       a short static string derived from workflow.description (v1: just echo the
       workflow name as the reason; richer reasoning is a v2 concern, not required here).

6.5  [T] crates/pidgin-core/src/tests/router_tests.rs
       Test: routing the generic_review workflow recommends "claude-code", falls back
       to "opencode", matching configs/WORKFLOW_REGISTRY.yaml exactly.

6.6  [G] cargo test -p pidgin-core router_tests   → passes.

6.7  [F] crates/pidgin-core/src/context.rs (Part 9.3)
       Implement:
         pub struct ContextPlan { pub primary_refs: Vec<String>, pub retrieval_methods: Vec<String>,
             pub token_budget: usize, pub compression_allowed: bool }
         pub fn build_context_plan(packet: &PgnPacket, resolved: &[ResolvedRef],
             budgets: &TokenBudgets) -> ContextPlan
       Body: primary_refs = the packet's `in` list verbatim; retrieval_methods =
       ["direct_path", "host_retriever"] as a v1 static default; token_budget =
       budgets.default_context_max_tokens; compression_allowed = true.

6.8  [T] One test confirming build_context_plan on the resolvable_task example
       produces token_budget == 8000 (from configs/TOKEN_BUDGETS.yaml, Part 2.7).
6.9  [G] cargo test -p pidgin-core context_tests   → passes.

6.10 [F] crates/pidgin-core/src/metrics.rs (Part 9.4)
       Implement:
         pub fn estimate_tokens(text: &str) -> usize { (text.chars().count() + 3) / 4 }
         pub struct TokenSavingsReport { pub pgn_tokens: usize, pub verbose_tokens: usize,
             pub savings_ratio: f32 }
         pub fn compare_verbose(pgn_text: &str, verbose_text: &str) -> TokenSavingsReport

6.11 [T] Test: estimate_tokens("abcd") == 1. Test: compare_verbose with a 40-character
       Pidgin packet and a 400-character verbose-prompt string produces
       savings_ratio > 0.85 (sanity bound, not an exact-value assertion, since the
       exact ratio depends on the literal example strings chosen).
6.12 [G] cargo test -p pidgin-core metrics_tests   → passes.

6.13 [F] crates/pidgin-cli/src/output.rs
       Implement a shared OutputFormat enum {Json, Yaml, Pretty} and a single
       render<T: Serialize>(value: &T, format: OutputFormat) -> String function used
       by every subcommand from this point forward, replacing the Phase-1-only
       {:#?} debug printing in `parse` with real JSON/YAML output. Update
       commands/parse.rs to use this.

6.14 [F] crates/pidgin-cli/src/commands/{context_plan,measure,compare}.rs
       Implement each per its Part 10.2 description, each a thin composition of the
       core functions already built in 6.4–6.12.

6.15 [F] crates/pidgin-cli/src/commands/run.rs
       Implement `run <file> [--host .] [--dry-run] [--execute]`. The full pipeline,
       in this exact order (mirrors Part 5's layer diagram precisely):
         1. parse_packet
         2. validate_syntax + validate_schema  (abort with exit 1 if either fails)
         3. resolve_all                         (resolver needs to run before the
            upgraded SG-4/SG-8 checks, so resolution happens before the final safety
            pass, even though some safety checks like SG-1/SG-2/SG-5/SG-6 do not
            need it — running resolution once, before safety, is simpler and no less
            correct than splitting safety into two passes)
         4. check_safety                        (abort with exit 2 if blocked)
         5. expand_to_run_packet                (abort with exit 3 if a required
            input is Missing per SG-8 — note expand_to_run_packet already enforces
            this per Phase 5; this is a defensive double-check at the CLI layer)
         6. build_context_plan
         7. estimate_tokens on both the raw packet text and the serialized RunPacket
         8. route
         9. log_runtime_run (Phase 7, once it exists; until Phase 7 lands, skip this
            step — Phase 6 can ship logging as a no-op stub that Phase 7 fills in)
         10. print a human-readable dry-run report (run_id, workflow, mode, risk,
             human approval, validation PASS, safety PASS, references resolved N/M,
             forbidden actions, expected outputs, token estimates, route, status: READY)
       If --dry-run (the default — Part 11.1's `default_dry_run: true`) or if
       --execute was NOT passed: stop here. No file is written outside the logs
       directory, no executor is invoked, no further action occurs.
       If --execute was passed: this is intentionally NOT implemented in Phase 6.
       Executor invocation is out of MVP scope (Part 14) — `--execute` should exist
       as a flag that, for now, prints "execution is not yet implemented; remove
       --execute to see the dry-run report" and exits 0. This keeps the boundary
       between "the protocol runtime" and "an actual agent executor" honest and
       explicit rather than half-implemented.

6.16 [G] cargo run -p pidgin-cli -- run examples/basic/resolvable_task.pgn
         --host examples/fixture_workspace --dry-run
       → prints the full report described in step 10 above, with Status: READY,
       and exits 0. Confirm no files were written anywhere except (once Phase 7
       lands) the logs directory. This is the Phase 6 exit gate.
       git add -A && git commit -m "phase 6: full CLI surface, host contract, dry-run pipeline complete"
```

### 13.10 Phase 7 — Logging and Metrics Persistence (Part 15)

**Goal:** every dry run and every safety block writes a row to the appropriate CSV log, append-only, never losing a row on crash.

```text
7.1  [C] cargo add csv chrono -p pidgin-core
7.2  [F] crates/pidgin-core/src/logging.rs
       Define one struct per log file, matching Part 15's column lists exactly:
         pub struct RuntimeRunRow { pub timestamp: String, pub run_id: String,
             pub workflow: String, pub mode: String, pub risk: String,
             pub human_required: bool, pub status: String, pub pgn_tokens: usize,
             pub expanded_tokens: usize, pub fired_safety_rules: String }
         pub struct ProtocolErrorRow { pub timestamp: String, pub run_id: String,
             pub error_code: String, pub message: String }
         pub struct TokenUsageRow { pub timestamp: String, pub run_id: String,
             pub pgn_tokens: usize, pub expanded_tokens: usize, pub context_tokens: usize }
       Implement, for each row type, an append_row(path: &Path, row: &T) -> io::Result<()>
       that: opens the file in append mode (creating it with a header row if it
       doesn't exist yet), writes one CSV record, and calls .flush() explicitly before
       returning — the explicit flush is what satisfies SG-10/Part 5's "never lose a
       log row on crash" requirement, since an unflushed buffered writer can silently
       lose its last rows if the process is killed.

7.3  [T] crates/pidgin-core/src/tests/logging_tests.rs
       Test 1: appending two rows to a fresh temp file produces a file with exactly
       3 lines (1 header + 2 data rows).
       Test 2: appending a row to an existing file with a header does NOT duplicate
       the header — append_row must detect an existing header and skip writing it again.
       Test 3 (crash-safety smoke test): after append_row returns Ok, reading the file
       back immediately (without any extra explicit close/sync from the test) shows
       the just-written row — proving the flush actually happened synchronously rather
       than being deferred.

7.4  [G] cargo test -p pidgin-core logging_tests   → passes.

7.5  [F] Wire crates/pidgin-cli/src/commands/run.rs step 9 (Phase 6, 6.15) to
       actually call append_row for RuntimeRunRow on every dry run, and for
       ProtocolErrorRow whenever validation or safety produces an error/block.
7.6  [G] Re-run: cargo run -p pidgin-cli -- run examples/basic/resolvable_task.pgn
         --host examples/fixture_workspace --dry-run
       → confirm .pidgin/logs/PIDGIN_RUNTIME_RUNS.csv (under the
       fixture_workspace host root) now contains a new row reflecting this run.
       Then run the same command against unsafe_no_human.pgn and confirm
       PROTOCOL_ERRORS.csv gets a new row mentioning Sg2.
       This is the Phase 7 exit gate.
       git add -A && git commit -m "phase 7: append-only CSV logging wired into the run pipeline"
```
### 13.11 Phase 8 — Python SDK (Subprocess Wrapper, Part 3.4)

**Goal:** `pip install -e python/` works, and a Python script can call `pidgin_runtime.check("packet.pgn")` and get back a typed, validated result without ever touching subprocess plumbing itself.

```text
8.1  [C] mkdir -p python/pidgin_runtime
8.2  [F] python/pyproject.toml
       [project]
       name = "pidgin-runtime"
       version = "0.1.0"
       requires-python = ">=3.11"
       dependencies = ["pydantic>=2.6", "pyyaml>=6.0"]

8.3  [F] python/pidgin_runtime/models.py
       Define pydantic v2 models mirroring the Rust structs field-for-field:
         class ResolvedRef(BaseModel): original: str; namespace: str; ref_id: str;
             resolved_path: str | None; confidence: float; required: bool; status: str
         class SafetyResult(BaseModel): allowed: bool; blocked: bool;
             fired_rules: list[str]; human_required: bool; effective_risk: str
         class RunPacket(BaseModel): run_id: str; workflow: str; mode: str;
             risk_level: str; human_approval_required: bool;
             inputs: list[ResolvedRef]; expected_outputs: list[str];
             forbidden_actions: list[str]
       These exist so a Python caller gets IDE autocomplete and validation on the
       JSON the Rust binary emits — never a raw, untyped dict.

8.4  [F] python/pidgin_runtime/client.py
       Implement:
         import subprocess, json, shutil
         class PidginError(Exception): ...
         def _binary_path() -> str:
             path = shutil.which("pgn")
             if path is None: raise PidginError("pgn binary not found on PATH; build with `cargo build --release` and add target/release to PATH")
             return path
         def _run(*args: str) -> dict:
             result = subprocess.run([_binary_path(), *args, "--format", "json"],
                 capture_output=True, text=True)
             if result.returncode not in (0, 1, 2, 3):
                 raise PidginError(f"unexpected exit code {result.returncode}: {result.stderr}")
             return json.loads(result.stdout) if result.stdout else {}
         def check(packet_path: str, host: str = ".") -> "SafetyResult":
             data = _run("check", packet_path, "--host", host)
             return SafetyResult.model_validate(data)
         def expand(packet_path: str, host: str = ".", out: str | None = None) -> "RunPacket":
             args = ["expand", packet_path, "--host", host]
             if out: args += ["--out", out]
             data = _run(*args)
             return RunPacket.model_validate(data)
       NOTE: this step requires that Phase 6's CLI commands support a `--format json`
       flag emitting a single JSON object on stdout. If that flag does not yet exist
       on a given subcommand, add it now as part of this step — do not skip ahead.

8.5  [T] python/tests/test_client.py
       Using pytest, a fixture that builds the Rust binary once (or skips with a
       clear message if cargo is unavailable in the test environment), then:
       Test 1: check() on examples/basic/generic_task.pgn returns a SafetyResult
       with allowed=True.
       Test 2: check() on examples/basic/unsafe_no_human.pgn returns allowed=False
       and "Sg2" in fired_rules.
       Test 3: expand() on examples/basic/resolvable_task.pgn (with host pointed at
       the fixture workspace) returns a RunPacket whose human_approval_required is True.

8.6  [G] cd python && pip install -e . --break-system-packages && pytest
       → all tests pass. This is the Phase 8 exit gate.
       git add -A && git commit -m "phase 8: python SDK (subprocess wrapper), typed pydantic models"
```

### 13.12 Phase 9 — Watch Mode (Part 10.2, `pgn watch`)

**Goal:** dropping a new `.pgn` file into the host's `inbox` automatically triggers validate → expand → write-to-outbox → log, with no human intervention, while still respecting every safety rule (a blocked packet is logged as blocked, never silently expanded).

```text
9.1  [C] cargo add notify -p pidgin-core
9.2  [F] crates/pidgin-core/src/watcher.rs (or inline in the CLI's watch command —
       acceptable either way; recommend core, for testability)
       Implement:
         pub fn watch_inbox(host: &HostConfig, mut on_new_file: impl FnMut(&Path)) -> notify::Result<()>
       using notify::RecommendedWatcher in non-recursive mode on host.inbox, filtering
       events to Create/Modify on files ending in ".pgn", and invoking on_new_file for
       each one (debounced — wait ~300ms after the last event for a given path before
       firing, to avoid double-triggering on editors that write in two syscalls).

9.3  [F] crates/pidgin-cli/src/commands/watch.rs
       Implement `watch <folder> [--host .]`: call watch_inbox with a closure that
       runs the same pipeline as `run --dry-run` (Phase 6, 6.15, steps 1–9) for every
       new file, then additionally calls expand_to_run_packet and writes the result
       to host.outbox (this is the one behavioral difference from `run --dry-run`:
       watch mode always attempts expansion-on-success, since its whole purpose is
       hands-off automation, but it still never executes anything and still refuses
       to expand a blocked packet, per SG-8/Part 5).

9.4  [T] crates/pidgin-core/src/tests/watcher_tests.rs (integration-style, using
       a tempdir fixture)
       Test: start watch_inbox in a background thread against a tempdir, write a
       valid .pgn file into its inbox, assert within a 2-second timeout that the
       corresponding RUN_PACKET.yaml appears in the outbox. Test: write an unsafe
       .pgn file (e.g. the unsafe_no_human fixture), assert that NO file appears in
       the outbox and a row appears in PROTOCOL_ERRORS.csv instead.

9.5  [G] cargo test -p pidgin-core watcher_tests   → passes.
       Manual confirmation: cargo run -p pidgin-cli -- watch examples/fixture_workspace/inbox
         --host examples/fixture_workspace
       (in one terminal) then, in another terminal, `cp examples/basic/resolvable_task.pgn
       examples/fixture_workspace/inbox/` → confirm a RUN_PACKET.yaml appears in
       examples/fixture_workspace/generated/ within ~1 second.
       This is the Phase 9 exit gate.
       git add -A && git commit -m "phase 9: watch mode, hands-off inbox automation"
```

### 13.13 Phase 10 — Integration Tests, Golden Tests, Benchmarks, CI

**Goal:** the project has a green CI pipeline, a golden-test suite that would catch any accidental change to the expansion output, and baseline performance numbers against the targets in Part 16.

```text
10.1 [C] cargo add insta --dev -p pidgin-core
10.2 [F] crates/pidgin-core/src/tests/golden_tests.rs
       For each file in examples/basic/*.pgn that is expected to succeed, parse →
       validate → resolve (against examples/fixture_workspace) → expand, then
       insta::assert_yaml_snapshot!(run_packet). Run `cargo insta review` once to
       accept the initial snapshots into examples/expanded/*.snap — these become the
       golden files. Any future change to expansion logic that alters output now
       fails CI loudly instead of silently.

10.3 [G] cargo test -p pidgin-core golden_tests   → passes (after the one-time
       insta review/accept step above).

10.4 [C] cargo add criterion --dev -p pidgin-core
10.5 [F] benches/parse_bench.rs, benches/validate_bench.rs, benches/resolve_bench.rs,
       benches/expand_bench.rs — each a criterion benchmark over the
       resolvable_task.pgn example, run 1 and run 1000 times in a loop.
10.6 [G] cargo bench   → record the numbers; compare against Part 16.1's targets
       (parse < 2ms, validate < 5ms, resolve common refs < 50ms, expand < 10ms,
       1000-packet validation < 2s). If any target is missed by more than 2x, that
       is a signal to profile before continuing — not a hard blocker for Phase 10's
       gate, but it must be written down in PERFORMANCE.md either way.

10.7 [F] .github/workflows/ci.yml
       jobs: fmt (cargo fmt --check), clippy (cargo clippy -- -D warnings),
       test (cargo test --workspace), bench (cargo bench, non-blocking/informational),
       python-test (pip install -e python/ && pytest), each on a matrix of
       {ubuntu-latest, macos-latest, windows-latest} for the fmt/clippy/test jobs.

10.8 [G] Push to a Git remote and confirm the CI workflow goes green on all matrix
       legs. This is the Phase 10 exit gate, and also the MVP-complete gate (Part 14.1)
       — once Phase 10's gate passes, the project has shipped a complete, tested,
       documented MVP per the scope in Part 14.
       git add -A && git commit -m "phase 10: golden tests, benchmarks, CI green — MVP complete"
```
### 13.14 Phase 11 — Orchestrator Framework Adapters and MCP Server Exposure (v1 scope)

**Goal:** Pidgin can be dropped into LangGraph, CrewAI, AutoGen, and exposed as an MCP server, without any of those integrations touching the Rust core directly — everything in this phase is a thin adapter over the Phase 8 Python SDK.

```text
11.1 [F] python/pidgin_runtime/adapters/langgraph_nodes.py
       Implement, as plain functions usable as LangGraph node callables:
         def parse_tsl_node(state: dict) -> dict:   # expects state["packet_path"]
             ...calls client.check()... returns state with state["safety_result"] set
         def validate_tsl_node(state: dict) -> dict: ...
         def resolve_refs_node(state: dict) -> dict: ...
         def expand_tsl_node(state: dict) -> dict: ...
       Each node is a pure function over a dict, matching LangGraph's plain-function
       node contract, so no LangGraph-specific base class or decorator is required —
       a host can add these to a StateGraph with .add_node("validate_tsl", validate_tsl_node)
       and nothing else.

11.2 [T] python/tests/test_langgraph_nodes.py
       Test: validate_tsl_node({"packet_path": "examples/basic/generic_task.pgn"})
       returns a dict containing a "safety_result" key whose "allowed" is True.

11.3 [F] python/pidgin_runtime/adapters/crewai_tools.py
       Implement a thin CrewAI Tool subclass (or function decorated with @tool,
       depending on the CrewAI SDK version in use at integration time — check
       CrewAI's current tool-definition API before writing this file, since
       orchestrator-framework APIs change between releases) that wraps
       client.check() and client.expand() as two separate CrewAI tools an agent
       can call.

11.4 [F] python/pidgin_runtime/adapters/autogen_adapter.py
       Implement two plain Python functions matching AutoGen's function-calling
       tool-registration signature (a callable with a docstring AutoGen can
       introspect), one for check() and one for expand(), registered the same way
       any other AutoGen tool function would be.

11.5 [F] python/pidgin_runtime/mcp_server.py
       Implement a minimal MCP server (using the official MCP Python SDK) exposing
       exactly three tools: "pidgin_check", "pidgin_expand",
       "pidgin_resolve" — each a direct call into the Phase 8 client functions,
       with the tool's input schema being the packet file path (and optional host
       path) and the output schema matching the corresponding pydantic model from
       models.py. This makes Pidgin itself usable from any MCP-compatible host
       (Claude Desktop, Claude Code, Cursor, or any other MCP client) without that
       host needing to know Pidgin exists as anything other than "an MCP server
       that validates and expands structured task packets" — directly mirroring how
       MCP servers are described generically: a host application creates an MCP
       client session that maintains a stateful JSON-RPC channel with its own MCP
       server, and the server exposes tools the host can discover and call without
       custom per-integration code.

11.6 [T] python/tests/test_mcp_server.py
       Using the MCP Python SDK's test client, confirm the server lists exactly the
       three tools above and that calling "pidgin_check" with
       examples/basic/generic_task.pgn returns allowed=true in its tool result.

11.7 [G] pytest python/tests/   → all adapter and MCP server tests pass.
       Document each integration in its corresponding docs/*_INTEGRATION.md file
       (already stubbed as empty files in Phase 0; fill them in now with the actual
       usage shown in 11.1–11.6).
       This is the Phase 11 exit gate, and the v1-complete gate (Part 14.2).
       git add -A && git commit -m "phase 11: LangGraph/CrewAI/AutoGen adapters + MCP server — v1 complete"
```

### 13.15 Phase 12 — v2 Scope (Daemon, Real Tokenizers, A2A Bridge) — Not Required for v1, Documented for Continuity

```text
12.1  pidgin-daemon: a local-only HTTP API (POST /parse, /validate, /expand,
      /resolve, /measure, /dry-run, GET /health) using axum, bound to localhost only
      by default, never exposed publicly without an explicit, separately-documented
      opt-in flag and an auth token — this is a hard requirement, not a suggestion,
      given that MCP's own security guidance explicitly warns that arbitrary data
      access and code execution paths in a host-exposed server carry serious trust
      implications that implementors must carefully address.
12.2  Real per-model tokenizer support (Part 9.6), added as an optional cargo
      feature flag, never a default dependency.
12.3  An A2A bridge module: a function that takes a Pidgin RunPacket and wraps
      it as the payload of an A2A Task object, for hosts that need to hand a
      Pidgin-validated instruction across an organizational/trust boundary using
      A2A's own Task lifecycle and transport — useful precisely because A2A defines
      its own task lifecycle states (submitted, working, input-required, completed,
      failed, canceled, rejected) and Pidgin's RunPacket is a natural, already-
      validated payload to carry inside one of those tasks rather than re-deriving
      validation on the other side of the boundary.
12.4  An ACP bridge module, analogous to 12.3, wrapping a RunPacket's content as one
      of ACP's typed multimodal Parts within its REST-style message envelope.
12.5  WASM build of pidgin-core (via wasm-pack) so the parser/validator can run
      client-side in a browser-based packet editor, with the safety gate and
      resolver excluded from the WASM target (they need real filesystem access and
      make no sense sandboxed in a browser).
12.6  TypeScript SDK, mirroring the Python SDK's subprocess-wrapper pattern from
      Phase 8 exactly, for hosts whose orchestration layer is Node/TypeScript-based.
```
## 14. Scope Summary (MVP / v1 / v2)

### 14.1 MVP (= Phases 0–7, Part 13.3–13.10)

```text
Rust CLI            parser            validator           safety gate
basic resolver       packet expander    token estimator     dry-run
CSV logs             basic config       tests at every phase gate
```

Explicitly **not** in MVP:

```text
daemon / HTTP API     WASM build          TypeScript SDK
orchestrator adapters  DSPy optimization   real tokenizers
full context retrieval  executor execution  network calls of any kind
```

### 14.2 v1 (= Phase 8–11, Part 13.11–13.14, built immediately after MVP)

```text
Python SDK (subprocess wrapper)      watch mode
LangGraph / CrewAI / AutoGen adapters    MCP server exposure
golden test suite                     performance benchmarks
CI across three OS targets
```

### 14.3 v2 (= Phase 12, Part 13.15, deferred, documented for continuity only)

```text
daemon mode (localhost-only HTTP API)     real tokenizer support
A2A bridge module                          ACP bridge module
WASM parser build                          TypeScript SDK
plugin system for custom NamespaceResolvers
visual run inspector (a small local web UI reading the CSV logs)
```

---

## 15. Performance Targets and Tactics

### 15.1 Targets

```text
Parse one packet:              < 2 ms
Validate one packet:           < 5 ms
Resolve common refs:           < 50 ms (cold), < 5 ms (cached)
Expand packet:                 < 10 ms
Dry run, simple packet:        < 100 ms end to end
Validate 1000 packets:         < 2 seconds
CLI cold start:                < 100 ms
```

These are deliberately the same numbers as the original draft — they were already well-chosen, conservative targets for a Rust CLI doing in-memory text processing with no network calls, and changing them would be change for its own sake rather than improvement.

### 15.2 Tactics

```text
Rust core, zero-copy parsing where winnow's API allows it (winnow operates over &str
  slices rather than owned String allocations wherever a borrow suffices — this is a
  direct benefit of choosing winnow over a framework that forces eager allocation)
lazy file reads (never read a file until a reference to it actually needs resolving)
registry caching (load WORKFLOW_REGISTRY.yaml once per process, not once per packet)
path alias cache, keyed by blake3 file hash, invalidated only on real file changes
parallel directory scans and parallel multi-packet validation via rayon
JSONL append for very high-volume logs if CSV row-locking ever becomes a bottleneck
  (v2 concern; CSV is sufficient and more human-debuggable at MVP scale)
no LLM calls anywhere in pidgin-core, ever (SG-11)
no network calls anywhere in pidgin-core by default, ever (Part 2, Part 13.15.1)
```

### 15.3 Why these specific numbers are achievable, not aspirational

A 2-ms parse target on a packet of roughly 10–15 short lines is not an ambitious target for a `winnow`-based parser — `winnow` was forked from `nom` specifically to fix a performance cliff that existed in certain `nom` usage patterns, and `nom` itself is already widely regarded as faster than most parser-generator alternatives such as `pest`. A packet this small, parsed with a combinator library built for exactly this throughput class, should be well under a millisecond in practice; the 2ms target leaves comfortable headroom for the surrounding allocation and error-handling overhead.
## 16. Testing Strategy

### 16.1 Test categories and where they live (cross-referenced to the build phases that create them)

| Category | Created in | Lives at | Purpose |
|---|---|---|---|
| Unit tests | every phase | `crates/pidgin-core/src/tests/*.rs` | One behavior, one assertion, fast |
| Safety rule tests | Phase 3 | `safety_tests.rs` | One test per `SG-n` rule, named after the rule (Part 13.6) |
| Golden/snapshot tests | Phase 10 | `golden_tests.rs` + `examples/expanded/*.snap` | Catch unintended drift in expansion output |
| Resolver fixture tests | Phase 4 | `resolver_tests.rs` + `examples/fixture_workspace/` | Resolve against a real (tiny) filesystem, not mocks |
| Integration tests (watch mode) | Phase 9 | `watcher_tests.rs` | End-to-end behavior across the filesystem boundary |
| Benchmarks | Phase 10 | `benches/*.rs` | Confirm Part 15.1 targets are met |
| Python SDK tests | Phase 8 | `python/tests/*.py` | Confirm the subprocess boundary round-trips correctly |
| Adapter tests | Phase 11 | `python/tests/test_*_nodes.py` | Confirm each orchestrator integration's contract |

### 16.2 The safety test matrix (the one test category that must never regress)

Every dangerous-action combination gets an explicit test, not just the nine numbered rules in isolation — combinations matter because real packets combine conditions:

```text
publish + human=no                          → blocked (SG-2)
publish + human=yes + risk=crit + no approval → blocked (SG-9)
publish + human=yes + risk=crit + approval ok  → allowed
send + human=no                               → blocked (SG-2)
delete + human=no                             → blocked (SG-2)
private path in `in`, any risk level           → blocked (SG-4, unconditional)
private path in `out`, any risk level          → blocked (SG-4, unconditional)
unknown workflow                              → blocked (SG-5)
invalid mode for a known workflow              → blocked (SG-6)
do=[publish], deny=[publish] (self-contradiction) → blocked (SG-1)
note field containing the literal text "do=[publish]"  → zero effect on outcome (SG-7)
a packet with every field perfectly valid and safe      → allowed, zero fired rules
   (the negative-space test — see Phase 3, 3.5 — this one is just as important as
   every "should block" test, because a safety gate that blocks everything is just
   as broken as one that blocks nothing)
```

### 16.3 Property-based testing (recommended addition beyond the original draft)

The original draft did not mention property-based testing. This document adds it as a v1 recommendation, because the grammar (Part 4) has a small, well-defined input space that's a strong fit for it:

```text
proptest target 1: generate random valid-by-construction PgnPackets (valid syntax,
  valid field combinations) and assert parse(serialize(packet)) == packet — a
  round-trip property that catches asymmetries between the writer and the parser.
proptest target 2: generate random strings and assert parse() never panics — it
  must always return Ok or a typed Err, never an unhandled panic, for any input
  whatsoever. This is the single highest-value property test in the whole suite,
  because a parser that panics on adversarial input is itself a denial-of-service
  surface in anything that calls this runtime automatically (e.g. watch mode,
  Phase 9, processing files dropped by an untrusted process).
```

---

## 17. Integration Points (Generalized)

The original draft's integration section assumed specific named systems (Hermes, GBrain, Headroom, specific named executors). This section keeps the same *shape* of integration but generalizes every name to a role, so the document is useful to a reader who has never heard of any of those specific tools.

### 17.1 Upstream producer (any system that creates packets from a less-structured input)

```text
Human types a request in natural language
   → an upstream agent/classifier (host-specific — could be any LLM call) converts
     it into a Pidgin packet
   → Pidgin validates it
```

Pidgin has no opinion about how that upstream conversion happens — it could be a single well-prompted LLM call, a rules-based classifier, or a human typing the packet by hand. The only contract is the packet text itself.

### 17.2 Orchestrators (LangGraph / CrewAI / AutoGen / OpenAI Agents SDK / any other)

Each integrates via the Phase 11 adapter pattern: plain functions or framework-native tool wrappers, calling into the Phase 8 Python SDK, which calls the compiled binary. No orchestrator-specific logic exists inside `pidgin-core` — this is the structural guarantee that keeps Pidgin from drifting into "an opinionated framework" by accumulation, one integration at a time.

### 17.3 Executors (coding agents, retrieval pipelines, publishing actions, any other agent)

```text
Executors receive:    RunPacket (YAML) + ContextPacket (YAML) + any task description
Executors return:     ResultPacket (YAML), validated against RESULT_PACKET_SCHEMA.json
                       on the way back in (Part 9.2's validate_result_packet)
```

Pidgin never invokes an executor directly in MVP/v1 scope (Part 14.1–14.2) — `--execute` is deliberately a stub through v1 (Phase 6, 6.15). This is a scope boundary worth defending explicitly: the moment Pidgin starts invoking executors itself, it has become an orchestrator, which Part 1.3 explicitly rules out.

### 17.4 Context/retrieval backends (vector stores, knowledge graphs, plain grep, or nothing)

Pidgin's `ContextPlan` (Part 9.3) is a request for retrieval, not retrieval itself. A host wires its own retrieval backend to consume a `ContextPlan` however it likes — this generalizes the original draft's specific named integrations (a particular vector database, a particular graph tool, a particular long-term memory store) into "any backend that can read a small, typed YAML plan."

### 17.5 Structured-output validation tooling (Pydantic / Instructor / Outlines)

```text
Pydantic — the Python SDK's models.py (Phase 8) IS this integration; no separate
  adapter needed since pydantic v2 is the SDK's own foundation.
Instructor — a host using Instructor to get structured LLM output can target the
  same pydantic models from models.py directly as Instructor's response_model,
  meaning an LLM call can be made to emit a RunPacket-shaped or ResultPacket-shaped
  object directly, then validated through the normal SDK path.
Outlines — similarly, a host using Outlines for constrained generation can use the
  JSON Schema files in schemas/ (Part 6, the actual source of truth) directly as
  Outlines' grammar/schema input, guaranteeing an LLM's raw output is already
  schema-conformant before Pidgin ever sees it.
```

### 17.6 Optimization tooling (DSPy or similar)

A host using DSPy to optimize a natural-language-to-Pidgin-packet conversion step, or a retrieval-query-rewriting step, treats Pidgin's `compare` command (Part 9.4) output as one of its optimization signals — the lower the token cost of the resulting packet (relative to a verbose baseline) while still passing validation and safety, the better the optimized prompt. This is a host-side optimization loop entirely external to `pidgin-core`; the runtime only needs to keep producing fast, honest token measurements for the loop to consume.

---

## 18. Packaging and Distribution

### 18.1 Rust binary

```bash
cargo build --release
# binary at target/release/pgn
```

### 18.2 Python package

```bash
pip install pidgin-runtime
# or, for local development:
pip install -e python/ --break-system-packages
```

### 18.3 Release artifacts (per tagged version)

```text
pgn-linux-x86_64
pgn-linux-aarch64
pgn-macos-aarch64
pgn-macos-x86_64
pgn-windows-x86_64.exe
pidgin-runtime (Python wheel, sdist)
source tarball
SHA256 checksums file
```

### 18.4 Install script (optional, v1+)

```bash
curl -fsSL https://<host>/install.sh | sh
```

For personal/local use, `cargo install --path crates/pidgin-cli` is sufficient and is the recommended path during MVP development — a public install script is a v1-or-later convenience, not a build-blocking requirement.

### 18.5 Licensing recommendation

The original draft never specified a license. This document recommends **Apache-2.0** (rather than plain MIT), for one specific, research-backed reason: every major agent-interoperability protocol referenced throughout this document — A2A, MCP, and the broader Linux Foundation/Agentic AI Foundation governance umbrella both protocols now sit under — uses Apache-2.0 as a deliberate adoption-friendly, patent-grant-inclusive choice, and A2A specifically is maintained as an open-source project under the Apache 2.0 license. Matching that convention costs nothing and slightly eases any future integration work that touches those protocols directly (Part 13.15, the v2 A2A/ACP bridge modules), since Apache-2.0-to-Apache-2.0 dependency chains have one fewer compatibility question to answer than a mixed MIT/Apache chain would.
## 19. Development Tooling Summary

### 19.1 Rust

```text
cargo            clippy           rustfmt
winnow           serde / serde_yaml / serde_json
jsonschema       clap             thiserror / anyhow
walkdir          ignore           rayon
notify           blake3
insta            criterion        proptest
cargo-audit      cargo-deny       (supply-chain security — Part 16, security.yml)
```

### 19.2 Python

```text
pydantic v2      typer (optional CLI convenience)
pytest           ruff (lint + format in one tool)
mypy             pyyaml           rich
```

### 19.3 CI checks (every push, per Phase 10's ci.yml)

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo bench               # informational, not blocking
cargo audit                # supply-chain vulnerability scan
pytest python/tests/
ruff check python/
mypy python/
```

### 19.4 Recommended task runner (`justfile`, created in Phase 0)

```just
build:
    cargo build --workspace

test:
    cargo test --workspace
    cd python && pytest

check: 
    cargo fmt --check
    cargo clippy --workspace -- -D warnings
    cargo test --workspace

bench:
    cargo bench

install-local:
    cargo install --path crates/pidgin-cli
    pip install -e python/ --break-system-packages
```

A `justfile` is recommended over a `Makefile` here for one concrete reason relevant to the "small local model" goal of this whole document: `just` recipes have no tab-vs-space whitespace sensitivity and no implicit shell-escaping rules to get subtly wrong, which removes an entire class of frustrating, hard-to-diagnose failure that `make` is notorious for when generated or edited by a model rather than a human who already has the footguns memorized.

---

## 20. Full End-to-End Worked Example (Replaces the Original Heap & Stack–Specific Walkthrough)

### 20.1 The packet

```text
@run review.module_auth
wf=generic_review
mode=review
in=[file:src/auth.rs,file:CONTRIBUTING.md]
out=[review_notes,risk_flags]
do=[read,review,flag]
deny=[publish,send,delete,secrets]
risk=med
human=yes
note="First pass on the new auth module before merge"
```

### 20.2 Command

```bash
pgn run review.module_auth.pgn --host . --dry-run
```

### 20.3 Output

```text
Pidgin Dry Run
------------------
Run ID: review.module_auth
Workflow: generic_review
Mode: review
Risk: med
Human approval: yes

Validation: PASS
Safety: PASS (fired_rules: [])
References: 2/2 resolved
  file:src/auth.rs        -> resolved, confidence 1.0
  file:CONTRIBUTING.md    -> resolved, confidence 1.0
Forbidden actions: publish, send, delete, secrets
Expected outputs: review_notes, risk_flags

Estimated packet tokens: 58
Estimated expanded RunPacket tokens: 211
Estimated context plan tokens (budget): 8000

Route:
  recommended_executor: claude-code
  fallback: opencode
  human_required: true

Status: READY
No external actions performed.
```

### 20.4 Artifacts written

```text
.pidgin/logs/PIDGIN_RUNTIME_RUNS.csv     (one new row appended)
.pidgin/logs/TOKEN_USAGE_LOG.csv             (one new row appended)
(nothing in outbox yet — `run --dry-run` does not write RUN_PACKET.yaml;
 use `pgn expand` or `pgn watch` for that, per Part 10.2 and Part 13.12)
```

### 20.5 The generic human workflow

```bash
# 1. Write a packet
nvim .pidgin/inbox/review.module_auth.pgn

# 2. Check it (fast feedback loop, no side effects)
pgn check .pidgin/inbox/review.module_auth.pgn --host .

# 3. See the full picture before committing to anything
pgn run .pidgin/inbox/review.module_auth.pgn --host . --dry-run

# 4. Expand it for a real executor to pick up
pgn expand .pidgin/inbox/review.module_auth.pgn --host . \
  --out .pidgin/generated/review.module_auth.RUN_PACKET.yaml
```

### 20.6 The generic agent/orchestrator workflow

```text
Upstream producer (human, classifier, or upstream agent) creates a Pidgin packet
   ↓
Pidgin validates + safety-gates it
   ↓
Orchestrator (LangGraph/CrewAI/AutoGen/etc., via the Phase 11 adapters) receives the
  validated RunPacket and ContextPlan
   ↓
Host's own retrieval backend builds the actual context, following the ContextPlan
   ↓
Executor (any agent) runs the task
   ↓
ResultPacket is validated on the way back in
   ↓
Optional, always human-gated, memory-promotion candidate is created
```
## 21. Final Feature Checklist

### Core (Phases 1–7)

- [ ] Parse Pidgin packets (Phase 1)
- [ ] Validate syntax and schema (Phase 2)
- [ ] Check safety, all 9 numbered rules tested independently and in combination (Phase 3, Part 16.2)
- [ ] Resolve references, with real-path-based private-path blocking (Phase 4)
- [ ] Expand packets to RunPacket/ContextPacket/ApprovalPacket (Phase 5)
- [ ] Estimate tokens, relative-comparison framing (Phase 6)
- [ ] Log every run and every error, append-only, explicitly flushed (Phase 7)
- [ ] Dry run, zero side effects outside the log directory (Phase 6)

### CLI (Phase 6)

- [ ] init · parse · validate · check · expand · resolve
- [ ] context-plan · measure · compare · run --dry-run · watch · doctor

### Integrations (Phase 8–11)

- [ ] Python SDK (typed, pydantic-backed, subprocess wrapper)
- [ ] LangGraph node adapters
- [ ] CrewAI tool adapters
- [ ] AutoGen function adapters
- [ ] MCP server exposure (pidgin as an MCP tool provider)
- [ ] Instructor/Outlines schema compatibility (via shared JSON Schema files)
- [ ] DSPy-compatible token-comparison signal (via `pgn compare`)

### Safety (Phase 3–4)

- [ ] Private path block (real, resolved-path-based — not string matching)
- [ ] Dangerous (human_gated) action block without human approval
- [ ] Critical-risk approval-packet requirement (SG-9)
- [ ] Denied action always overrides requested action (SG-1)
- [ ] No secrets ever resolved or logged
- [ ] Free-text `note` field never parsed for instructions (SG-7, tested explicitly)
- [ ] Strict, documented exit codes (Part 10.3)
- [ ] Negative-space test: a fully valid, safe packet produces zero fired rules

### Performance (Phase 10)

- [ ] Registry cache (load once per process)
- [ ] Alias/resolution cache (blake3-keyed, invalidated on real file change)
- [ ] Parallel multi-packet validation (rayon)
- [ ] Benchmarks recorded against Part 15.1 targets
- [ ] Fast CLI cold start (single static binary, no interpreter startup)

---

## 22. Final Recommendation

Build Pidgin as:

```text
Rust core, Rust CLI (single binary, no required runtime dependency)
Python SDK as a thin, typed subprocess wrapper from day one
host-agnostic configuration (the "host contract," Part 6.1) — never coupled to one vault
CSV/JSONL logs, append-only, explicitly flushed
dry-run as the default and the primary command
a strict, exhaustively-tested, fail-closed safety gate
orchestrator integrations as thin adapters, never as core logic
MCP-server exposure so any MCP-compatible host can use it without custom integration
A2A/ACP bridges as opt-in v2 modules, not core dependencies
```

Do not build it as:

```text
a full agent framework
a model gateway
a memory database
a publishing system
a competitor to MCP, A2A, ACP, or ANP — it is a layer beneath all of them
a prompt-only convention with no enforcement
```

The runtime should be boring, fast, strict, and almost invisible when it's working. The power comes from this, restated from the original draft because it was already exactly right and needed no improvement, only a home that isn't tied to one vault's name:

```text
Short Pidgin handoff
→ strict validation
→ safe expansion
→ targeted, plan-only retrieval
→ lower context usage
→ cleaner agent-to-agent and human-to-agent communication
→ auditable, append-only logs
```

**Final rule, generalized:**

```text
Pidgin should make any system's agents communicate less, mean more, and break less —
regardless of which vault, framework, or vendor sits on either side of it.
```

---

## 23. References and Further Reading

This document's research claims are grounded in the following sources, gathered during the research pass that produced this document (June 2026). A small model or human extending this project should treat these as the starting bibliography for any future protocol-design decision, not as a closed, final list.

**Agent interoperability protocols**

1. Linux Foundation. *Agent2Agent (A2A) Protocol* — governance, v1.0 status, adoption milestones. `https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents` and `https://www.linuxfoundation.org/press/a2a-protocol-surpasses-150-organizations-lands-in-major-cloud-platforms-and-sees-enterprise-production-use-in-first-year`
2. A2A technical architecture and JSON-RPC error model. `https://tyk.io/learning-center/a2a-protocol-architecture-and-technical-specification/`
3. A2A Agent Cards, Task lifecycle, and transport model. `https://atlan.com/know/google-a2a-protocol/`
4. Anthropic. *Model Context Protocol specification.* `https://modelcontextprotocol.io/specification/2025-11-25` and `https://github.com/modelcontextprotocol/modelcontextprotocol`
5. MCP architecture, adoption, and AAIF governance transition. `https://en.wikipedia.org/wiki/Model_Context_Protocol` and `https://dev.to/x4nent/complete-guide-to-mcp-model-context-protocol-in-2026-architecture-implementation-and-4a11`
6. IBM. *Agent Communication Protocol (ACP)* overview and design principles. `https://macronetservices.com/agent-communication-protocol-acp-ai-interoperability/`
7. ACP-IBM message schema (roles, multimodal Parts). Sun et al., *Beyond Message Passing: A Semantic View of Agent Communication Protocols.* `https://arxiv.org/pdf/2604.02369`
8. Comparative survey of MCP, ACP, A2A, ANP layering. `https://arxiv.org/html/2505.02279v1` and `https://arxiv.org/pdf/2506.05364`
9. Security threat modeling across MCP, A2A, Agora, ANP. `https://arxiv.org/pdf/2602.11327`

**Token economics and compact agent communication (the research case for the protocol's grammar design)**

10. *Token Economics for LLM Agents: A Dual-View Study from Computing and Economics* — message-level compression, CodeAgents structural reformatting. `https://arxiv.org/html/2605.09104v1`
11. *LDP: An Identity-Aware Protocol for Multi-Agent LLM Systems* — compact payload formats as a governance and cost requirement. `https://arxiv.org/html/2603.08852`
12. *CodeAgents: A Token-Efficient Framework for Codified Multi-Agent Reasoning in LLMs.* `https://arxiv.org/html/2507.03254v1`
13. *Agent Q-Mix: Selecting the Right Action for LLM Multi-Agent Systems through Reinforcement Learning* — token-efficiency benchmark figures. `https://arxiv.org/pdf/2604.00344`
14. *AgentDropout: Dynamic Agent Elimination for Token-Efficient and High-Performance LLM-Based Multi-Agent Collaboration.* `https://arxiv.org/pdf/2503.18891`

**Rust parser tooling**

15. Comparative analysis of PEG (pest) vs. parser-combinator (nom) approaches. `https://www.synacktiv.com/en/publications/battle-of-the-parsers-peg-vs-combinators`
16. *Winnow 0.5: The Fastest Rust Parser-Combinator Library?* — design goals, fork rationale relative to nom. `https://epage.github.io/blog/2023/07/winnow-0-5-the-fastest-rust-parser-combinator-library/`
17. Winnow design philosophy versus chumsky and nom. `https://docs.rs/winnow/latest/winnow/_topic/why/index.html`

> **Note on currency:** several of the above sources are dated within the same month as this document's writing (June 2026); the agent-protocol landscape is moving quickly, and anyone picking this project back up after a gap of more than a few months should re-run the equivalent research pass before treating Part 1.2's protocol-comparison table as current.
