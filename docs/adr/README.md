# Architecture decision records

One file per decision that is expensive to reverse. Format: status, date,
context, decision, alternatives, consequences. Superseded records stay in place.

| # | Decision | Status |
| --- | --- | --- |
| [0001](0001-rust-and-gtk4.md) | Rust and GTK4 + libadwaita | accepted |
| [0002](0002-local-embeddings-and-classifier.md) | Local embeddings plus an online classifier | accepted |
| [0003](0003-sqlite-catalog.md) | One SQLite file as the catalog | accepted |
| [0004](0004-gpl3.md) | GPL-3.0-or-later | accepted |
| [0005](0005-workspace-boundaries.md) | Workspace crate boundaries | accepted |
| [0006](0006-read-only-first.md) | Read-only first, safe mutations later | accepted |
| [0007](0007-ignore-aware-scanning.md) | Ignore-aware scanning and foreground watch | proposed |
| [0008](0008-operation-engine.md) | One operation engine with an undo journal | proposed |

## Proposed next

These are not decided yet. Open an ADR before implementing.

- **Model distribution**: bundled weights versus a pinned download, and the
  offline story for Flatpak.
- **Content extraction boundary**: which formats, where extracted text lives,
  and how to exclude secrets.
- **Event storage**: same SQLite file versus an append-only log, and retention.
- **Search result grouping**: whether grouping is presentational or changes the
  ranking contract.
- **Settings schema**: versioning, unknown-key preservation and migration.
- **App-level index encryption**: key management, or explicitly rely on
  full-disk encryption.
