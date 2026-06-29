# CLI Reference

## `pgn init`

Scaffold `.pidgin/` directory with default configuration files.

```
pgn init
```

Creates `.pidgin/` in the current working directory with five YAML files, each populated with default values. Safe to re-run — will not overwrite existing files.

## `pgn parse <path>`

Lex and parse a `.pgn` packet file, printing the typed AST to stdout.

```
pgn parse examples/basic/generic_task.pgn
```

Output shows each token and the structured `PgnPacket` with all fields, including the header (directive + run_id) and each typed field.

## `pgn validate <path>`

Syntax + schema validation.

```
pgn validate examples/basic/generic_task.pgn
```

Checks field presence against the directive schema and field types against allowed values. Exits 1 on any violation.

## `pgn check <path>`

Full end-to-end guard: validate → safety gate → resolve references.

```
pgn check examples/basic/unsafe_contradiction.pgn
pgn check examples/basic/unsafe_no_human.pgn
pgn check examples/basic/unsafe_private_path.pgn
```

Runs all pipeline stages except expansion and logging. Best for quick safety reviews. Exits 0 if clean, 2 if safety blocks.

## `pgn resolve <path>`

Expand all short references to their real paths/IDs.

```
pgn resolve examples/basic/generic_task.pgn
```

Prints the packet with every reference resolved against `REFERENCE_ALIASES.yaml` and the filesystem. Unresolved required references cause exit 3.

## `pgn expand <path>`

Full pipeline: parse → validate → safety → resolve → expand.

```
pgn expand examples/basic/generic_task.pgn
```

Produces a fully-specified YAML packet with every field expanded, every reference resolved, and safety annotations. The output is what an executor would act on.

## `pgn run <path>`

Same as `expand` plus structured logging.

```
pgn run examples/basic/generic_task.pgn
```

Appends the expanded packet to `.pidgin/logs/` with a timestamp and run_id. This is the production command.

## `pgn measure <path>`

Estimate token cost of a packet.

```
pgn measure examples/basic/generic_task.pgn
```

Prints a breakdown: header tokens, field tokens, list tokens, reference tokens, total. Uses a simple estimation model (blanket tokens = 0.25 * packet_content_length, packet tokens = length / 4).

## `pgn compare <path>`

Compare Pidgin token cost vs equivalent verbose description.

```
pgn compare examples/basic/generic_task.pgn
```

If the packet has a `note` field, that note is used as the verbose baseline. Otherwise, a verbose description is generated. Shows % savings.

## `pgn context-plan <path>`

Build a structured context-retrieval plan from the packet's `in` references.

```
pgn context-plan examples/basic/generic_task.pgn
```

Each reference is annotated with its resolved type and a retrieval strategy. This plan can be passed to a retrieval layer.

## `pgn doctor`

Check host configuration.

```
pgn doctor
```

Verifies `.pidgin/` exists, each config file is valid YAML with the expected keys, and basic path resolution works. Useful for debugging host setup.

## `pgn docs`

Print full protocol documentation as markdown to stdout.

```
pgn docs > pidgin-protocol.md
```

Intended for agent consumption — agents can pipe this into their context window to understand Pidgin protocol without leaving their environment.

## `pgn --help` / `pgn -h`

Print usage summary with all available commands.

## `pgn --version` / `pgn -V`

Print the installed version.
