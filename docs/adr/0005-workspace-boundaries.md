# ADR 0005 — Workspace crate boundaries

- Status: accepted
- Date: 2026-10-23

## Context

The engine, the CLI and the GUI evolve at different speeds. GTK headers should
not be required to run tests or index files on a server, and retrieval logic
should not be reachable only through a widget.

## Decision

A Cargo workspace with five crates and a strict dependency direction:

```
folderol-core   (types, tokenization, category priors; no I/O except stat)
     ▲
folderol-ml     (embedder trait, hashing embedder, classifier)
     ▲
folderol-index  (SQLite catalog, FTS5, hybrid retrieval)
     ▲
folderol-cli    folderol-gui
```

Rules:

- Dependencies point only upward in that diagram. No cycles.
- GTK types may not appear below `folderol-gui`.
- No crate opens a socket. Model downloads, when added, live in a separate
  opt-in crate/binary.
- `folderol-gui` is excluded from `default-members` so `cargo build` and
  `cargo test` work without GTK development packages.
- Tests live next to the code they test. Cross-crate behaviour is tested through
  the CLI or an integration test in the highest relevant crate.

## Alternatives considered

- **One crate with modules.** Faster to start, but the UI boundary erodes and
  headless CI grows GTK dependencies.
- **Separate repositories.** Version skew and cross-repo changes for a project
  this size; rejected.
- **A service/daemon with a thin client.** Solves incremental indexing for
  multiple frontends, but adds lifecycle, IPC and security surface before the
  product needs it. Revisit only if background indexing proves necessary.

## Consequences

- Compile times stay reasonable and changes are scoped.
- Public APIs of the lower crates are effectively internal contracts; they may
  change before 1.0 without semver ceremony.
- The GUI crate is the only place that needs a display, which keeps CI cheap.
- New capabilities get a home: `folderol-extract` for content, `folderol-watch`
  for inotify, `folderol-thumbs` for glycin, each depending only on what it
  needs.
