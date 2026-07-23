# Publishing Ownership

Pidgin has one core contract: `pidgin-lang` owns PGN grammar, validation, safety,
resolution, expansion, and protocol semantics. Everything else wraps it.

## Core

Published and versioned together:

- `pidgin-lang`
- `pidgin-cli` / `pgn`

## Core-adjacent

Published as one Python package:

- `pidgin-python`

It contains the Python runtime wrapper and the optional native PyO3 source under
`python/pidgin-python/native`. It must not introduce separate grammar or safety
semantics.

## Adapters

Published independently under `python/adapters/`:

- `pidgin-pydantic`
- `pidgin-langgraph`
- `pidgin-crewai`
- `pidgin-deepagents`

Adapters are active ecosystem packages. They may grow based on user demand, but
they delegate protocol behavior to core/runtime.

## Wrapper

Published independently:

- `pidgin-server`

The server is a decoupled wrapper/demo surface. It is not required by the core
protocol and stays outside the default workspace.
