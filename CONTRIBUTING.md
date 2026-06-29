# Contributing to Pidgin

Thanks for wanting to contribute. This project is young and there's a lot to build. Here's how you can help.

## What Needs Doing

### High Priority

- **More workflows in the default registry** — The default `WORKFLOW_REGISTRY.yaml` has three workflows. Real deployments need more: incident response, content syndication, approval chains, data pipelines.
- **New safety rules** — The gate has nine rules. Can you think of SG-10? Common patterns: "same agent is both producer and reviewer", "refers to a resource the caller doesn't own", "risk level mismatches historical average for this workflow".
- **Python SDK** — The `python/` directory has scaffolding. Fill it in: PyO3 bindings or a proper subprocess wrapper with typed Pydantic models.
- **Log rotation** — Logs are append-only with size/retention limits in config. Implement actual rotation (archive + truncate when `max_log_size_mb` is hit).
- **Configuration schema validation** — Config YAML files are deserialized but not deeply validated. Add schema checks that give users clear error messages when their config has typos.

### Medium Priority

- **`pgn serve`** — A subcommand that runs Pidgin as a lightweight HTTP server (just enough to accept a packet and return expanded YAML). Useful for Docker deployments.
- **MCP server implementation** — The README shows a Python MCP server sketch. A production version in Rust using the `mcp-server` crate would be faster and not require Python.
- **Proptest expansion** — Add property-based tests for the expander, router, and metrics stages (currently only parser has them).
- **Exit code 5 reporting** — Internal errors should include a unique error ID and suggest the user file a bug with the full output.

### Low Priority / Exploration

- **A2A interop example** — A complete worked example showing Pidgin ↔ A2A handoff with a real external agent.
- **`pgn replay`** — Replay a logged handoff from the `.pidgin/logs/` directory, re-running the pipeline and showing what changed.
- **WebAssembly target** — The parser and safety gate could compile to WASM for use in browser-based agent UIs.
- **`pgn lint`** — Check `.pgn` files for style issues (field ordering conventions, missing optional but recommended fields).

## Getting Started

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/pidgin.git
cd pidgin

# Build and test
cargo build
cargo test
cargo clippy -- -D warnings

# Run against examples
cargo run -- parse examples/basic/generic_task.pgn
cargo run -- check examples/basic/unsafe_contradiction.pgn
```

## Design Principles

1. **No LLM in the hot path.** Every pipeline stage is deterministic. If a feature would require a model call, it doesn't belong in core.
2. **Fail closed.** When uncertain, block. No "warn but proceed" paths.
3. **Flat over nested.** The packet grammar is flat key=value lines. Config YAML should be shallow.
4. **Audit everything.** Every pipeline stage writes to the log. If it's not logged, it didn't happen.
5. **Minimal dependencies.** No HTTP clients, async runtimes, or TLS in core. Keep the dependency tree small for `cargo audit`.

## Pull Request Guidelines

- One change per PR. If you have two unrelated improvements, open two PRs.
- Add tests. New safety rules need at least one passing and one blocking test case.
- Update docs. If you change behavior, update the relevant `docs/` file and the crate README.
- Run `cargo test && cargo clippy -- -D warnings && cargo audit` before submitting.
- Keep the commit history clean. No merge commits, no "fix typo" fixups.

## Code of Conduct

Be respectful. This is a small project run by one person — I'll treat every contribution seriously and expect the same in return.

## Questions?

Open a [GitHub Discussion](https://github.com/manojpisini/pidgin/discussions) or file an issue. I try to respond within 48 hours.
