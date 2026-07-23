# Pidgin Protocol Specification

This document describes the Pidgin packet grammar, runtime architecture, safety model, and configuration — everything you need to understand how it works and how to mount it in your own system.

---

## The Packet Grammar

A Pidgin packet is plain text: one header line followed by key-value field lines. Nothing nested, nothing optional about the structure — just a flat list of typed fields.

### Header

```
@<directive> <run_id>
```

`run_id` is a dotted identifier like `task.example` or `my.workflow.2026-06-28`. Four directives exist:

| Directive | What It Means |
|-----------|--------------|
| `@run` | A task to execute |
| `@result` | The outcome of a task |
| `@approval` | Human sign-off for a critical-risk task |
| `@context` | A request for additional context |

### Fields

Every field is `key=value` on its own line. Whitespace around the `=` is not allowed.

```
field_name=scalar_value
field_name=[val1,val2,val3]
```

Scalars can be:
- **Bare words**: `risk=med`, `wf=generic_review`, `human=yes`
- **Quoted strings**: `note="Draft only, do not publish"` (for values with spaces or special characters)
- **Numbers**: `ttl=24` (not widely used yet)
- **Booleans**: `human=yes`, `human=no`

Lists are comma-separated inside square brackets:

```
in=[primary_subject,source_refs]
in=[file:src/main.rs,file:src/lib.rs]
```

Comments start with `#` and must be on their own line — inline comments are not supported.

### Required vs Optional Fields

| Field | `@run` | `@result` | `@approval` | `@context` |
|-------|--------|-----------|-------------|------------|
| `wf` | required | required | optional | required |
| `mode` | required | — | — | optional |
| `in` | required | — | optional | required |
| `out` | required | required | — | optional |
| `do` | optional | — | — | — |
| `deny` | optional | — | — | — |
| `risk` | optional | — | required | optional |
| `human` | optional | — | required (yes) | — |
| `ttl` | optional | — | optional | optional |
| `note` | optional | optional | optional | optional |
| `status` | — | required | required | — |
| `produced` | — | required | — | — |

### Reference Syntax

Inside lists and the `route` field, references come in two forms:

```
namespace:id          — explicit namespace + identifier
bare_alias            — resolved through REFERENCE_ALIASES.yaml
```

Built-in namespaces: `ep`, `rb`, `ledger`, `claim`, `policy`, `skill`, `wf`, `file`, `folder`, `dash`, `queue`.

Examples: `file:src/main.rs`, `ep:UNIT012`, `policy:community_reply`.

---

## Runtime Architecture

The pipeline is linear. Every stage is a pure function — same input always produces same output, no side effects, no IO except reading config files at the start.

```
packet.pgn
    │
    ▼
┌─────────────┐
│   Lexer     │  winnow tokenizer
└─────────────┘
    │
    ▼
┌─────────────┐
│   Parser    │  → PgnPacket (typed AST)
└─────────────┘
    │
    ▼
┌──────────────────┐
│ Syntax Validator  │  required fields? correct types?
└──────────────────┘
    │
    ▼
┌──────────────────┐
│ Schema Validator  │  workflow/mode/risk legal against registries?
└──────────────────┘
    │
    ▼
┌──────────────────┐
│   Safety Gate     │  SG-1 through SG-9
└──────────────────┘
    │  (blocked packets stop here)
    ▼
┌──────────────────┐
│ Ref Resolver      │  short refs → real paths/IDs
└──────────────────┘
    │
    ▼
┌──────────────────┐
│  Packet Expander  │  → executable YAML packet
└──────────────────┘
    │
    ▼
┌──────────────────┐
│  Context Planner  │  what to retrieve
└──────────────────┘
    │
    ▼
┌──────────────────┐
│  Metrics          │  token cost estimate
└──────────────────┘
    │
    ▼
┌──────────────────┐
│  Router           │  recommended executor
└──────────────────┘
    │
    ▼
 expanded packet (YAML)
```

### What Each Stage Must Never Do

| Stage | Must Never |
|-------|-----------|
| Parser | Guess a missing field |
| Syntax Validator | Apply business rules (that's schema's job) |
| Schema Validator | Touch the filesystem (that's resolver's job) |
| Safety Gate | Be bypassed by a packet's own declared `risk` or `human` |
| Resolver | Treat an unresolved required ref as resolved |
| Expander | Make network calls |
| Context Planner | Actually retrieve anything (it only plans) |
| Metrics | Block execution (it only informs) |
| Router | Execute anything (it only recommends) |
| Logging | Lose a row on crash (append + flush) |

---

## Safety Gate (SG-1 through SG-9)

The safety gate is the reason Pidgin exists. Every rule exists because I saw a real-world failure pattern that could have been caught before it caused damage.

| Rule | What It Catches | Why |
|------|----------------|-----|
| **SG-1** | Action appears in both `do` and `deny` | Contradictory instructions must never be silently resolved. If you say both "do publish" and "deny publish", something is wrong. |
| **SG-2** | Human-gated action without `human=yes` | Publishing, deleting, sending credentials — these require explicit human sign-off. The default is to block. |
| **SG-3** | High/critical risk with `human=no` | You cannot opt out of human review for high-risk actions. The packet's own declaration is not trusted over the registry. |
| **SG-4** | Reference resolves to a private path | `.env`, `.ssh/`, `*.pem`, `**/secrets/**` — these are blocked regardless of risk level or human approval. |
| **SG-5** | Unknown workflow `wf` | An unrecognized workflow is not a no-op. It's a bug or an attack. |
| **SG-6** | `mode` not in workflow's allowed modes | A review workflow should never execute in publish mode. |
| **SG-7** | `note` field parsed for instructions | The note field is the most obvious prompt-injection surface. It is never parsed by any layer. |
| **SG-8** | Required input ref failed to resolve | Running with missing inputs produces silent failures downstream. Better to block and ask. |
| **SG-9** | Critical risk without approval packet | `risk=crit` requires a separate `@approval` packet with `status=ok`. A single boolean on the same packet is not enough. |

**The principle**: if any stage is uncertain, it blocks. Every violation is logged with the rule ID and the triggering values.

---

## Host Configuration

Pidgin is host-agnostic. Any system — a Git repo, a vault folder, a CI pipeline — can mount it by providing a `.pidgin/` directory with these files:

| File | What It Defines |
|------|----------------|
| `PIDGIN_RUNTIME_CONFIG.yaml` | Host paths, runtime mode, default deny list |
| `WORKFLOW_REGISTRY.yaml` | Each workflow: risk default, allowed modes, required inputs, expected outputs, recommended executor |
| `ACTION_REGISTRY.yaml` | Three tiers: `safe` (always allowed), `controlled` (allowed with review), `human_gated` (requires human=yes) |
| `SAFETY_RULES.yaml` | Default deny list, private path patterns, human-required actions and risk levels |
| `REFERENCE_ALIASES.yaml` | Short names for frequently-used references, so packets can say `source` instead of `file:src/main.rs` |

Run `pgn init` to scaffold all five files with sensible defaults.

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation error (syntax or schema) |
| 2 | Safety gate blocked (one or more SG-n rules fired) |
| 3 | Required reference unresolved |
| 4 | Configuration error (missing paths, malformed YAML) |
| 5 | Internal error (should never happen — file a bug) |

---

## Multi-Agent Integration

Pidgin does not run agents. It validates and expands the handoffs *between* them.

```
LangGraph / CrewAI / Custom orchestrator
         │
         │ produces .pgn packet
         ▼
    Pidgin runtime (validate → safety → resolve → expand)
         │
         │ expanded YAML packet
         ▼
    Executor agent (Claude, Codex, custom tool)
         │
         │ produces result.pgn
         ▼
    Pidgin runtime (validate → log)
         │
         ▼
    Orchestrator reads result
```

The Python runtime (in `python/pidgin-python/`) wraps the binary via subprocess or the bundled native source, giving orchestrators typed objects instead of CLI output. Framework adapters live under `python/adapters/` and must delegate protocol behavior back to core/runtime.
