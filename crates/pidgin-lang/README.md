# Pidgin

**A compact protocol runtime for agent-to-agent handoffs.** Parse, validate, safety-check, resolve, expand, and log structured messages between agents — with no LLM in the hot path and no network calls in the core.

[![Crates.io](https://img.shields.io/crates/v/pidgin-lang.svg)](https://crates.io/crates/pidgin-lang)
[![Docs.rs](https://img.shields.io/docsrs/pidgin-lang)](https://docs.rs/pidgin-lang)

---

## Install

```bash
cargo install pidgin-lang
pgn init
pgn --help
pgn docs
```

## What It Does

Pidgin sits between agents and ensures every handoff is parseable, valid, safe, and logged. A packet is nine lines of `key=value` text:

```text
@run task.example
wf=generic_review
mode=draft
in=[primary_subject,source_refs]
out=[review_notes]
do=[draft,review]
deny=[publish,send,delete]
risk=med
human=yes
```

The runtime pipeline: **parse → validate (syntax + schema) → safety gate (9 rules) → resolve references → expand to executable YAML → log**.

Every stage is a public function. Every safety rule fails closed. No network calls. No LLM calls.

## Commands

| Command | What It Does |
|---------|-------------|
| `init` | Scaffold `.pidgin/` with default configs |
| `parse` | Parse a `.pgn` file, print AST |
| `validate` | Syntax + schema validation |
| `check` | Full end-to-end safety + resolution check |
| `resolve` | Expand short references |
| `run` | Full pipeline (parse → validate → safety → resolve → expand) |
| `measure` | Token cost estimation |
| `docs` | Full protocol documentation |

Exit codes: `0` success, `1` validation error, `2` safety blocked, `3` unresolved ref, `4` config error.

## Safety

Nine rules (SG-1 through SG-9) catch contradictions, missing human approval, private path access, unknown workflows, invalid modes, unresolved required refs, and more. Every violation is logged. Every uncertain case blocks rather than guesses.

## Library

```rust
use pidgin_lang::parser::parse_packet;
let packet = parse_packet("@run my.task\nwf=generic_review\nmode=draft")?;
```

## Links

- [Docs.rs](https://docs.rs/pidgin-lang)
- [GitHub](https://github.com/manojpisini/pidgin)
- [crates.io](https://crates.io/crates/pidgin-lang)
