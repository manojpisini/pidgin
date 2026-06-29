# Security Policy

## Reporting a Vulnerability

I take security seriously. If you find a vulnerability in Pidgin, please report it privately so it can be fixed before public disclosure.

**Contact:** You can reach me directly on the [Pidgin GitHub repository](https://github.com/manojpisini/pidgin/security/advisories) via the private advisory feature, or open an issue with a "security" label if you're unsure whether something qualifies.

I aim to acknowledge reports within 48 hours and ship a fix within a week of confirmation.

## Scope

The following are in scope:
- The Rust runtime (`pidgin-lang` crate on crates.io)
- The `pgn` CLI binary
- Default configuration files and their parsing

Out of scope:
- Host systems or vaults that consume Pidgin (they should mount Pidgin as documented)
- Python SDK (scaffolded, not yet shipped)
- Build-time dependencies (cargo-audit runs in CI)

## Known Security Properties

Pidgin is designed with these guarantees:

1. **No network calls in core** — The runtime never makes HTTP requests or opens sockets by default. It is local-first by construction.
2. **No LLM in the hot path** — Every pipeline stage is deterministic and does not call any model. The safety gate cannot be bypassed by a packet's own declarations.
3. **Fail closed** — If any stage is uncertain (unresolved reference, unknown workflow, ambiguous packet), it blocks rather than guessing. A packet must pass every stage to reach expansion.
4. **Path containment** — File references are canonicalized and checked against the host root. The resolver rejects traversal attempts (`../../etc/passwd`), symlink escapes, and encoded variants.
5. **Input limits** — Packets are capped at 1 MB and 100 fields. Fields are capped at 10,000 characters. Config YAML files are capped at 10 MB.
6. **Sanitized logging** — All user-controlled values in log output are filtered to printable ASCII only, with length limits, preventing log injection and CSV injection.

## Supported Versions

Only the latest published version on crates.io receives security patches. I do not backport fixes to older versions.
