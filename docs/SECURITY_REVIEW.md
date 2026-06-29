# Security Review

This document covers how I thought about security when building Pidgin — the threat model, the design decisions that matter, and why certain trade-offs were made.

---

## Threat Model

Pidgin is not a network service. It reads a file from disk, validates it, and either blocks or produces an expanded version. The threat model is accordingly narrow:

1. **Malicious packet** — A crafted `.pgn` file that tries to bypass the safety gate, access private files, or inject behavior through fields like `note`.
2. **Compromised config** — A host's `.pidgin/` directory is tampered with to weaken safety rules or add dangerous workflow definitions.
3. **Log injection** — The packet contains values that, when written to a log file, corrupt the log or execute embedded commands when read by a tool.
4. **Supply chain** — The crate's dependencies contain a vulnerability.

What is NOT in scope: network attacks, denial of service, side channels, physical access, or attacks requiring write access to the filesystem where Pidgin is running (if you already have that, the game is over).

---

## Defenses by Layer

### 1. The Lexer

The lexer rejects packets larger than 1 MB and packets with more than 100 fields. Individual fields are capped at 10,000 characters. These limits prevent memory exhaustion and pathological parsing scenarios.

The `note` field is tokenized as a raw string value. It is never interpreted as a field, list, or directive. This is the first line of defense against prompt injection — the note can contain anything, and the grammar actively refuses to parse it.

### 2. The Safety Gate (SG-1 to SG-9)

The safety gate is a separate stage that runs after parsing and validation but before resolution and expansion. It cannot be disabled by a packet's own fields. If a packet declares `risk=high` and `human=no`, the gate still blocks it (SG-3) — the packet's self-declaration is not trusted.

SG-4 (private path resolution) uses canonicalized absolute paths to check against denylist patterns. Symlinks are followed and their targets are canonicalized before the check.

SG-7 (note isolation) is structural: the `note` field is stored as an opaque string that no subsequent stage inspects.

SG-9 (critical risk approval) requires a separate `@approval` packet. A single `human=yes` on a crit-risk packet is not enough — the pattern requires two distinct messages.

### 3. Reference Resolution

All `file:` and `folder:` references are canonicalized via `std::fs::canonicalize`. The resulting absolute path is checked against the host root directory. Traversal attempts (`../../etc/passwd`), symlink escapes, and encoded variants (`%2e%2e%2f`) are all caught at this stage because canonicalization resolves them to their target, which will not be under the host root.

### 4. Logging

Log entries are append-only. Each line is flushed and the file handle is closed after every write. This prevents log loss on crash — the last completed write is always durable.

User-controlled values (packet fields, resolved paths) are sanitized before writing:
- Non-printable characters are replaced with their hex escape
- Length is capped at 500 characters per value
- Newlines in values are escaped to prevent CSV/log injection

### 5. Dependency Analysis

CI runs `cargo audit` on every push. Current advisories: zero. The dependency tree is kept deliberately small — no HTTP clients, no async runtime, no TLS libraries. The heaviest external dependency is the YAML parser.

---

## Design Decisions

### No LLM in the Pipeline

The most important decision. Pidgin does not call any model, ever. A model could be used to make the safety gate "smarter" — interpreting ambiguous fields, resolving fuzzy references, deciding if a path is private. But that would turn every decision into a probability. Pidgin's safety gate is deterministic: either the rule fires or it doesn't. If the runtime is uncertain, it blocks.

### Fail Closed

Every stage defaults to blocking. If a workflow is unknown, the packet is rejected (SG-5). If a required reference fails to resolve, the packet is rejected (SG-8). If the safety gate encounters any error during rule evaluation, it treats the rule as violated. There is no "warn but proceed" path.

### Host Agnosticism

Pidgin does not know what vault, CMS, or orchestrator it is running inside. It reads a directory of YAML files and validates packets against them. This means the host's identity cannot be used to weaken safety — Pidgin does not trust the host's branding, only its configuration files.

---

## Remaining Risks

1. **Config file integrity** — If an attacker can write to `.pidgin/SAFETY_RULES.yaml`, they can disable any rule. Defense: file permissions on the config directory. Future work: config signing.

2. **Side-channel in resolution time** — A reference that resolves to a very large remote path might take longer to resolve, leaking information about the filesystem structure. Defense: all resolution is local and bounded by input limits.

3. **Wildcard in private path patterns** — `"**/secrets/**"` could miss a path like `Secrets/` on a case-sensitive filesystem. Defense: patterns are checked case-sensitively. Hosts on case-insensitive filesystems should add both-case variants.

4. **Log file growth** — A high volume of packets could fill the disk. Defense: `max_log_size_mb` and `log_retention_days` in the runtime config. Log rotation is on the roadmap.

---

## Reporting

Found something I missed? See [SECURITY.md](../SECURITY.md) for the reporting process.
