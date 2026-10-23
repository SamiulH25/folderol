# ADR 0007 — Ignore-aware scanning and foreground watch

- Status: proposed
- Date: 2026-10-23

## Context

Folderol must index a real home directory without becoming a cache indexer. It
must also keep up with changes without installing a background service. The
choices here shape how trustworthy the catalog feels and how much of the user's
machine it touches.

Constraints:

- Developers already have `.gitignore` files; duplicating that syntax is a bug.
- A scan of 100k files must not re-read unchanged files.
- A watched home directory can produce thousands of events per minute.
- The application must remain usable if watching is unavailable.
- No daemon, no root, no network.

## Decision

**Scanning.** Use the `ignore` crate (the ripgrep engine) with:

- `.gitignore`, `.git/info/exclude`, global git excludes and `.ignore` inside
  repositories;
- `.folderolignore` everywhere, including outside repositories;
- hidden entries skipped by default;
- a short default-exclude list for build/tool caches (`node_modules/`,
  `target/`, Python caches), never for `build/` or `dist/`;
- symlinks not followed by default;
- filesystem boundaries not crossed by default;
- a 512-row transactional batch, an `AtomicBool` cancellation flag, and an
  unchanged test of path + size + mtime + vector presence.

**Watching.** `folderol watch` is a foreground process using `notify`/inotify
with a 400 ms debounce and a 5 s hard batch cap. Events map to single-file
indexing, subtree scans for new directories, and tree removal for deletes and
renames. The watcher is not a service and has no autostart. The GUI will embed
the same engine in M3.

**Exclusions are absolute.** The catalog database and its WAL files are never
indexed, even when the database lives inside a watched root. Ignore files are
re-checked per event so watch and scan agree.

## Alternatives considered

- **Custom walker with a hand-written gitignore matcher.** Full control, but
  subtle bugs in negation, nested files and character classes are guaranteed.
  Rejected.
- **`walkdir` plus a separate ignore library.** Works, but duplicates the
  traversal and filter integration that `ignore` already tests. Rejected.
- **Always-on daemon with a socket.** Better multi-client behaviour and
  background indexing, but adds lifecycle, permissions, upgrade and security
  surface before the product needs it. Revisit only if GUI and CLI must watch
  simultaneously.
- **Polling every N seconds instead of inotify.** Simple and portable, but
  expensive on 100k files and slow to reflect changes. Kept as a manual refresh,
  not a default.
- **Cross-filesystem scanning by default.** Surprising on machines with network
  mounts; can stall on an unavailable share. Opt-in instead.
- **Following symlinks by default.** Can escape the chosen root and create
  cycles. Rejected; opt-in with cycle detection.

## Consequences

- Watch and scan share one filter implementation, which is the only way their
  results stay consistent.
- `.folderolignore` gives users a project-independent escape hatch.
- Default excludes are a product opinion. Each addition needs a test and a
  release note, and `build/`/`dist/` are deliberately not excluded.
- Ignore-file changes can trigger an expensive parent-tree rescan. The hard cap
  and debounce bound it; a more surgical invalidation is an open question.
- inotify watch limits become a visible failure mode. The UI must explain the
  limit rather than silently stop updating.
- A partial index is valid. Cancellation commits what it has and tells the
  truth; there is no rollback.
- No daemon means no background indexing while Folderol is closed. This is
  acceptable for 1.0 and documented.
