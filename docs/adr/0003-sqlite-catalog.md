# ADR 0003 — One SQLite file as the catalog

- Status: accepted
- Date: 2026-10-23

## Context

The catalog must store file metadata, text for full-text search, float vectors,
human labels and (later) interaction events. It must survive crashes, support
incremental updates, be inspectable, and be trivial to back up or delete.

## Decision

A single SQLite database at `$XDG_DATA_HOME/folderol/index.sqlite3`, opened in
WAL mode, with FTS5 for lexical search and vectors stored as little-endian `f32`
BLOBs. Schema migrations use `PRAGMA user_version`.

Retrieval at v0 scans all vectors in memory (brute-force cosine). `sqlite-vec`
is the planned ANN layer at M2; the schema does not need to change to adopt it.

## Alternatives considered

- **JSON / custom files.** No transactions, no query planner, no FTS, easy to
  corrupt on crash. Rejected.
- **sled / LMDB / RocksDB.** Fast key-value stores, but every query shape
  (category counts, joins to labels, FTS) would be hand-built. Rejected.
- **Tantivy for lexical + a separate vector store.** Excellent search, but two
  indexes to keep consistent and two failure modes. Revisit only if FTS5 becomes
  the bottleneck.
- **PostgreSQL / a local service.** A file manager must not require a daemon.
  Rejected.
- **In-memory only.** Cold start would re-scan the home directory. Rejected.

## Consequences

- Backup and "delete my data" are file operations, which is the right mental
  model for a local tool.
- FTS5 tokenization is Unicode-aware with diacritic folding. Queries are built
  from our own tokenizer, never from raw user input, which keeps FTS syntax
  injection out of the picture.
- Brute-force cosine sets a practical ceiling around a few hundred thousand
  files per query on one thread. That is acceptable for v0 and measured by the
  benchmark suite before `sqlite-vec` lands.
- SQLite is compiled in via the `bundled` feature, so the binary does not depend
  on the host's SQLite version or FTS5 build flags.
- The database contains full paths and is therefore sensitive: `0600`, no
  world-readable temp copies, and no content by default.
