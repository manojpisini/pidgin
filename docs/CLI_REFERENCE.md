# Pidgin CLI Reference

Binary name: `pgn`

## Commands

### `pgn init`

```bash
pgn init [--host .]
```

Creates the host's `config_dir` (default `.pidgin/`) with default registries, plus `inbox`, `generated`, and `logs` directories. Idempotent — running twice never overwrites existing customized registries.

### `pgn parse`

```bash
pgn parse <file> [--format json|yaml] [--pretty]
```

Outputs the parsed AST. Exit code `0` on success, `1` on a syntax error.

### `pgn validate`

```bash
pgn validate <file>... [--host .]
```

Runs syntax + schema validation only. Exit codes: `0` valid, `1` invalid.

### `pgn check`

```bash
pgn check <file> [--host .]
```

Runs validate → safety gate → resolve, end to end. "Tell me everything that's wrong, fast."

### `pgn expand`

```bash
pgn expand <file> [--host .] [--out <path>] [--packet run|approval|context]
```

Options: `--format yaml|json`, `--packet run|approval|context`.

### `pgn resolve`

```bash
pgn resolve <file> [--host .]
```

Prints every reference and its resolution status/confidence.

### `pgn context-plan`

```bash
pgn context-plan <file> [--host .] [--out <path>]
```

### `pgn measure`

```bash
pgn measure <file>
```

Shows token estimates for the raw packet.

### `pgn compare`

```bash
pgn compare <pgn-file> --verbose <verbose-file>
```

Shows the `TokenSavingsReport`.

### `pgn run`

```bash
pgn run <file> [--host .] [--dry-run] [--execute]
```

Runs the entire pipeline. `--dry-run` is the default — writes no files outside logs and performs no external actions.

### `pgn watch`

```bash
pgn watch <folder> [--host .]
```

Watches for new `.pgn` files → validate → expand → write to outbox → log.

### `pgn doctor`

```bash
pgn doctor [--host .]
```

Checks config files, log directories, schemas, and example packets.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | success |
| 1 | validation error (syntax or schema) |
| 2 | safety blocked (one or more SG-n rules fired) |
| 3 | reference missing/unresolved (and was required) |
| 4 | config error (host contract paths missing, registry malformed) |
| 5 | runtime/internal error (should never happen; always a bug report) |
