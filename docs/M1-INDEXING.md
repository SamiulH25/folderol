# M1 — A trustworthy index

**Status:** implementation specification. Nothing in this document is shipped
on `main` until the acceptance checklist passes; the document is the contract
the implementation is reviewed against.

**Milestone goal:** point Folderol at a real home directory, interrupt it,
restart it, watch it, and trust every number it reports.

## 1. Why this milestone exists

The catalog is only useful if the person believes it. A single phantom file, a
missing rename, an index that quietly stops updating, or a scan that re-reads
400,000 unchanged files destroys that belief faster than any missing feature.

M1 is therefore not a feature milestone. It is the correctness and operational
foundation the learning layer sits on.

## 2. Goals

- Ignore noise according to the same rules developers already understand.
- Make rescans incremental, interruptible and resumable.
- Keep the index live while the filesystem changes.
- Make the catalog private by default.
- Produce numbers that can be trusted: counts, errors, timings.
- Ship the engine as an installable application, not a `cargo run`.

## 3. Non-goals

- Content extraction. Names, paths and metadata only until M2.
- Automatic model download. A changed model forces a full rescan on the next run, not silently in the background.
- Background daemon. Watch runs in the foreground of `folderol watch`; the GUI
  embeds the same engine in M3.
- Network filesystems. They are walked if explicitly chosen, but no special
  reconnect or cache semantics are promised.
- Archive indexing. Archives are files, not containers, until 1.1.

## 4. Components

```text
                 ┌────────────────────────────────────────────┐
   roots ───────▶│ folderol-scan                              │
                 │  WalkBuilder (ignore crate)                │
                 │  filters · batching · cancellation         │
                 └───────────────┬────────────────────────────┘
                                 │ NewFile
                                 ▼
                 ┌────────────────────────────────────────────┐
                 │ folderol-index (SQLite)                    │
                 │  is_current · upsert_many · remove_tree    │
                 │  meta · labels · FTS5                      │
                 └───────────────┬────────────────────────────┘
                                 │
        ┌────────────────────────┴───────────────────────┐
        ▼                                                ▼
  folderol-cli scan/watch/bench                 folderol-gui (M3)
```

New crate: **`folderol-scan`**. It is the only crate allowed to walk user
directories. The CLI becomes a thin adapter; the GUI will call the same API.

Public API:

```rust
pub struct ScanOptions {
    pub include_hidden: bool,
    pub follow_links: bool,
    pub default_excludes: bool,
    pub cross_filesystems: bool,
    pub limit: Option<usize>,
    pub force: bool,
}

pub fn scan(
    catalog: &mut Catalog,
    roots: &[PathBuf],
    options: &ScanOptions,
    embedder: &dyn Embedder,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(&ScanStats),
) -> Result<ScanStats, ScanError>;

pub fn index_file(catalog: &mut Catalog, path: &Path, embedder: &dyn Embedder,
                  force: bool) -> Result<bool, ScanError>;
pub fn index_tree(catalog: &mut Catalog, root: &Path, embedder: &dyn Embedder)
                  -> Result<ScanStats, ScanError>;
pub fn remove(catalog: &mut Catalog, path: &Path) -> Result<usize, ScanError>;
pub fn prepare_file(record: FileRecord, embedder: &dyn Embedder) -> NewFile;
pub fn is_included(path: &Path, options: &ScanOptions) -> bool;
```

## 5. Scan pipeline

```text
1. canonicalise each root (absolute paths are the identity)
2. read meta.embedder_id
     if present and differs from embedder.model_id() → force = true
3. build walker:
     hidden = !include_hidden
     git_ignore = true, git_global = true, git_exclude = true
     require_git = true
     custom ignore filename = .folderolignore
     follow_links = options.follow_links
     same_file_system = !options.cross_filesystems
     overrides = default excludes (unless disabled)
4. for each entry:
     cancel requested? → flush, mark cancelled, stop
     not a regular file? → skip
     seen += 1
     stat record
     unchanged (path, size, mtime, embedding present) and not forced?
       → unchanged += 1, continue
     build NewFile (prior + feature text + embedding)
     batch += 1; flush every 512
     limit reached? → stop
5. flush final batch
6. if not cancelled: meta.embedder_id = model_id, meta.embedding_dim = dim
7. report stats
```

Design rules:

- **Batching is transactional.** A crash loses at most 512 files of work and
  never leaves a half-written row.
- **Progress is a snapshot, not a callback per file.** The CLI prints at most
  five times per second.
- **Errors are counted, printed once, and skipped.** One unreadable directory
  must not abort a 400k-file scan.
- **Stats count files, not directories.** `seen` is the number of regular files
  offered after filtering.

## 6. Ignore policy

Folderol uses the `ignore` crate, the same engine as ripgrep. The semantics are
deliberately familiar rather than bespoke.

### 6.1 Sources and precedence

Highest priority first:

| Priority | Source | Scope | Notes |
| --- | --- | --- | --- |
| 1 | Default excludes | every root | `node_modules/`, `target/`, tool caches; disable with `--no-default-excludes` |
| 2 | `.folderolignore` | directory and below | nested files override shallower ones; `!` re-includes |
| 3 | `.ignore` | directory and below | ripgrep-compatible |
| 4 | `.gitignore` | inside a git repository | only when a `.git` exists at or above the file |
| 5 | `.git/info/exclude` | repository | standard git semantics |
| 6 | Global git excludes | repository | `core.excludesFile` |
| 7 | Hidden filter | every root | dot entries skipped unless `--include-hidden` |
| 8 | Symlink filter | every root | links are not followed unless `--follow-links` |
| 9 | Filesystem boundary | every root | mount points skipped unless `--cross-filesystems` |

In practice the walker applies these together; the table describes intent, not
an implementation order. A negation in a deeper ignore file wins over a
shallower pattern, matching git.

### 6.2 Default excludes

```text
node_modules/
target/
__pycache__/
venv/
.venv/
.tox/
.mypy_cache/
.pytest_cache/
```

Rules for changing this list:

- A default exclude must be a build or tool cache, never a place a person stores
  deliverables.
- `build/`, `dist/`, `out/`, `bin/` are **not** excluded. They routinely contain
  the thing the user is looking for.
- Every addition needs a test proving it is skipped and a release note, because
  a silent exclusion looks like a missing file.

### 6.3 `.folderolignore`

Same syntax as `.gitignore`:

```gitignore
# Generated, never worth indexing
*.tmp
*.part
.cache/

# Keep this one searchable
!important.tmp
```

- Applies even outside a git repository.
- Nested files are supported; the deepest matching rule wins.
- Changing an ignore file triggers a rescan of its directory in watch mode.
- The file is never excluded itself if `--include-hidden` is set; otherwise it
  is a hidden file and is not indexed, which is fine.

### 6.4 Roots and boundaries

- Multiple roots are allowed and must not overlap. Overlapping roots are
  de-duplicated by canonical path; the shallower root wins.
- The catalog database, its `-wal`/`-shm` siblings, and `model.json` are always
  excluded, even when they sit inside a root.
- Cross-device traversal is off by default. `--cross-filesystems` opts in and
  prints how many mount points were entered.
- A root that does not exist is an error before the walk starts, with the exact
  path. One bad root does not stop the others if more than one was supplied;
  the exit code is 1 and the summary lists it.

### 6.5 Symlinks

- Default: the walker describes the link as a file entry? No — links are
  skipped entirely and never indexed as their target. The browser still shows
  them; the catalog does not claim they are regular files.
- `--follow-links`: the walker follows, and cycle detection is delegated to the
  ignore crate. The stats report `links_followed` (planned field).
- A symlink is never allowed to pull a scan outside a root. This is enforced by
  canonicalising entries and rejecting paths that do not start with a canonical
  root when following is enabled.

## 7. Incremental and resumable scans

### 7.1 The unchanged test

A file is skipped only when **all** hold:

1. the exact absolute path exists in `files`,
2. `size` matches,
3. `modified` matches (including both being `NULL`),
4. `embedding IS NOT NULL`.

This is a heuristic. It can miss an edit that preserves size and mtime to the
second, which is why:

- live changes are caught by the watcher,
- `--force` exists,
- a changed `embedder_id` forces everything.

The test is one indexed SQL query per file. At 100k files this is the difference
between a 40-second scan and a 0.8-second rescan.

### 7.2 Force

`--force` re-embeds and rewrites every file, preserving labels through the
upsert rule. It is required after:

- changing the embedder or its dimension,
- changing the feature-text template,
- a suspected corrupted vector,
- a bug fix that alters category priors.

The CLI prints the reason when it forces automatically (`embedder changed`).

### 7.3 Resume

There is no checkpoint file. An interrupted scan is resumed by running it again:

- written batches are already committed,
- unchanged files are skipped by §7.1,
- files not yet reached are indexed on the next run.

If the model changed and the scan was cancelled, `meta.embedder_id` is **not**
updated. Mixed old/new vectors therefore cannot masquerade as a complete
migration; the next run forces again.

### 7.4 Labels

Upsert preserves a human label:

```sql
category = COALESCE(
  (SELECT labels.category FROM labels WHERE labels.file_id = files.id),
  excluded.category
)
```

A rescan must never undo a correction. This is covered by a test in
`folderol-index`.

## 8. Cancellation and consistency

- The CLI installs a `SIGINT` handler that sets an `AtomicBool`.
- The scanner checks it once per entry, flushes the current batch, and returns
  `cancelled = true`.
- A second `SIGINT` exits immediately with code 130. The first cancellation is
  the safe path; the second is an escape hatch.
- Cancellation is a success, not an error: exit code 0, with a clear “partial
  index” summary and the number of files written.
- Nothing is rolled back. A partial index is valid and useful; rerunning
  completes it.
- The watcher shares the same handler and shuts down its inotify instance
  before exiting.

## 9. Watch mode

`folderol watch [PATHS]` runs an initial incremental scan, then applies
filesystem events until interrupted. It is a foreground process. There is no
service, no PID file, no autostart.

### 9.1 Backend

- `notify` 8.x with the inotify backend on Linux.
- One recursive watch per root.
- Debounce window: 400 ms after the last event in a batch.
- Hard batch cap: 5 s. A pathological stream of events must not postpone
  updates forever.
- Events are collected into a set of paths, then pruned: if a directory is in
  the set, its descendants are removed from the batch and handled by one tree
  operation.

### 9.2 Event mapping

| Event | Action |
| --- | --- |
| `Create` regular file | `index_file(path, force = false)` |
| `Modify(Data)` regular file | `index_file(path, force = true)` |
| `Modify(Metadata)` | `index_file(path, force = false)`; size/mtime may be unchanged |
| `Create` directory | `scan` the new subtree with current options |
| `Remove` file | `remove_tree(path)` |
| `Remove` directory | `remove_tree(path)` |
| `Modify(Name)` with one path | treat as remove (the old name is gone) |
| `Modify(Name)` with two paths (rename) | remove the old, index the new; a directory is rescanned |
| `Access` | ignored |
| `Other` / overflow | incremental rescan of the affected root |
| Watch error | print, keep running; after 5 consecutive errors, rescan the root |

### 9.3 Filtering

Each candidate path must pass:

1. not inside the catalog directory or the model path,
2. hidden filter respected,
3. default excludes respected,
4. `.folderolignore` / `.gitignore` respected (re-checked with a single-path
   walker so watch and scan agree),
5. still exists for add/modify events.

Deletion is always processed, even for a path that would now be ignored,
otherwise stale rows survive.

### 9.4 Known limitations (documented, not hidden)

- A change to `.gitignore` or `.folderolignore` triggers a rescan of its
  directory; a change in a **parent** ignore file triggers a rescan of that
  parent's tree. This is correct but can be expensive in a home directory.
- inotify has a per-user watch limit. The watcher reports the failing root and
  suggests `fs.inotify.max_user_watches`; it does not silently degrade.
- Network filesystems and FUSE may not emit events. The UI keeps a manual
  refresh, and `folderol watch` does not claim correctness there.
- Watcher ignore matching is per-event, so a burst of 10k events into one
  directory is slower than a scan. The hard cap bounds it.

### 9.5 Shutdown

- `Ctrl+C` stops accepting events, flushes the pending batch, closes the
  catalog, and exits 0.
- No partial operation is left behind: each event is a small transaction.
- If the database is locked by another process for more than 5 s, the watcher
  prints once and continues retrying on the next batch.

## 10. Catalog changes

Schema version 2 adds only:

```sql
CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

Keys: `embedder_id`, `embedding_dim`, `schema_note`.

New queries:

```sql
-- is_current
SELECT COUNT(*) FROM files
WHERE path = ?1 AND size = ?2 AND modified IS ?3 AND embedding IS NOT NULL;

-- remove_tree
SELECT id FROM files
WHERE path = ?1
   OR substr(path, 1, length(?1) + 1) = ?1 || '/';
```

`remove_tree` deletes FTS rows explicitly (FTS5 has no foreign key) and relies
on `ON DELETE CASCADE` for labels and events.

### 10.1 File permissions

- `harden_process_umask()` sets `umask(0o077)` once at startup.
- The catalog file is `0600`; the default data directory is created by the
  process under the hardened umask (`0700`).
- `model.json` and settings inherit the same umask.
- Folderol never changes permissions of an arbitrary parent directory. If
  `--db /tmp/x.sqlite3` is used, the file is private but `/tmp` is untouched.
- The database stores full paths and is therefore treated as a secret.

## 11. CLI contract

```text
folderol [--db PATH] <command>

scan [PATHS…]
  --include-hidden
  --no-default-excludes
  --follow-links
  --cross-filesystems
  --limit N
  --force
  --format human|json        (json planned)

watch [PATHS…]
  --include-hidden
  --no-default-excludes
  --follow-links
  --cross-filesystems
  --no-initial-scan

search <QUERY> [--limit N]
learn <PATH> --category SLUG
stats
categories
bench [--files N] [--queries N] [--assert-search-ms MS]
```

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | success, including a graceful cancellation |
| 1 | operational error (unreadable root, catalog failure, failed benchmark gate) |
| 2 | usage error (clap) |
| 130 | forced exit on the second `SIGINT` |

### Output

- Human results on stdout.
- Progress, warnings and errors on stderr, so `folderol search q > file` is
  clean.
- Progress format: one carriage-return line in a TTY, one line every 1,000
  files otherwise.
- Final scan summary includes: roots, files seen, indexed, unchanged, errors,
  elapsed, files/second, model id.
- `--format json` emits one object with the same fields, no progress lines.

## 12. Benchmarks

`folderol bench` builds a synthetic in-memory catalog and measures:

- batch insert + embedding time for `--files` (default 10,000),
- hybrid query latency across `--queries` (default 20) representative queries,
- p50, p95, max in milliseconds,
- vector memory estimate.

Queries cover: exact extension, two-word filename, semantic phrase, nonsense
token, single common word, scoped-looking text.

The benchmark is a regression gate, not a marketing number. CI runs it in
release mode with 20,000 files and a generous 150 ms p95 threshold to avoid
flaky failures; the real budget is 30 ms at 100k on the reference machine.

A second benchmark (manual, M1 exit) runs a real scan over a generated tree of
100,000 files with realistic names, on NVMe, and records wall time, peak RSS and
index size in `docs/benchmarks/2026-10-*.md`.

## 13. Test matrix

### Unit

- ignore precedence: default exclude beats nothing; `.folderolignore` negation
  re-includes; `.gitignore` only inside a repo.
- hidden filter, follow-links off, cross-filesystems off.
- unchanged test: path/size/mtime permutations and `NULL` mtime.
- label survives upsert and rescan.
- remove_tree removes exact path and descendants, not a sibling with a shared
  prefix (`/a/bc` must not match `/a/b`).
- FTS rows disappear with the file.
- DB mode `0600`.
- meta round-trip and schema migration 1 → 2.

### Integration

- scan a temp tree; assert counts and paths.
- scan twice; second is `unchanged`.
- touch a file; second scan indexes exactly one.
- cancel mid-scan via the flag; partial rows exist, meta not advanced.
- watcher: create, modify, rename, delete, directory create/delete, ignore file
  change, burst debounce.
- watcher does not index the catalog itself when the DB is under a root.
- kill -9 during a scan; reopen; rerun; counts converge.

### Manual

- `$HOME` with real dotfiles, a git repo, a node project, an external drive.
- inotify limit exhaustion.
- permission-denied subtree.
- read-only mount.
- 100k synthetic directory listing in the GUI (M3 dependency).

### Performance

As §12. CI stores the JSON output as an artifact for trend review.

## 14. Packaging

Files to add:

```text
data/io.github.samiulh25.Folderol.desktop
data/io.github.samiulh25.Folderol.metainfo.xml
data/icons/hicolor/scalable/apps/io.github.samiulh25.Folderol.svg
packaging/PKGBUILD
packaging/io.github.samiulh25.Folderol.yml   # Flatpak manifest
packaging/cargo-sources.json                 # generated, committed for Flatpak
```

- Desktop entry registers `inode/directory`, `%U`, and the New Window/Search
  actions.
- AppStream metadata includes summary, description, screenshots (once they
  exist), release notes and the GPL-3.0-or-later license.
- AUR package `folderol-git` builds from the git tag and installs both binaries,
  the desktop file, icons and metadata.
- Flatpak uses the GNOME 48 runtime or newer, `--filesystem=home`,
  `--share=ipc`, Wayland and fallback X11; **no network permission**.
- CI produces a tarball with `folderol`, `folderol-gui`, desktop file and
  license, plus SHA-256 checksums.

## 15. Migration from 0.1

| Change | Action |
| --- | --- |
| schema 1 → 2 | automatic `CREATE TABLE meta`; no data rewrite |
| paths stored relatively by `scan .` | 0.2 canonicalises roots. Old relative rows remain until the next scan; a `folderol doctor` action (planned) lists them. |
| `embedder_id` absent | first 0.2 scan writes it; vectors are hashing-v1 and match |
| removal of `walkdir` dependency | internal only |
| CLI `--db` unchanged | yes |

## 16. Acceptance criteria

M1 is done only when all are true:

- [ ] A 100k-file scan completes in ≤ 60 s on the reference NVMe machine.
- [ ] A second scan with no changes completes in ≤ 2 s and reports 0 indexed.
- [ ] Interrupting with `Ctrl+C` exits 0, reports partial counts and a rerun
      converges to the correct total.
- [ ] Changing `embedder_id` forces a full rebuild; cancelling it leaves the old
      marker in place.
- [ ] `node_modules/`, `target/`, hidden files and `.folderolignore` entries are
      absent from the catalog; `--include-hidden` and
      `--no-default-excludes` change exactly what they claim.
- [ ] Creating, editing, renaming and deleting a file while `watch` runs is
      reflected within 1 s.
- [ ] Deleting a directory removes every descendant and its FTS row.
- [ ] Human labels survive rescans and model changes.
- [ ] The catalog is `0600`; the default data directory is `0700`.
- [ ] No file content is read during a scan, proven by strace on a fixture.
- [ ] `folderol bench --files 20000` prints p50/p95 and fails when
      `--assert-search-ms` is exceeded.
- [ ] `cargo test --workspace`, clippy with `-D warnings`, and fmt are green.
- [ ] Arch package installs on a clean VM and launches from the application
      menu.
- [ ] Deleting the index leaves every user file untouched.

## 17. Task breakdown

| Task | State |
| --- | --- |
| `folderol-scan` crate, walker, batching, cancellation | WIP branch |
| `Catalog::is_current`, `remove_tree`, meta (schema 2) | WIP branch |
| DB and process permissions hardening | WIP branch |
| CLI rewired to `folderol-scan` | pending |
| `watch` command with debounce and event mapping | pending |
| `bench` command | pending |
| `--format json`, exit-code audit | pending |
| packaging, desktop entry, icon, AppStream | pending |
| benchmark report on generated 100k tree | pending |
| docs updates and ADR for watcher | pending |

## 18. Open questions

1. **Ignore-file changes in watch mode.** Rescanning the parent tree is correct
   but potentially expensive. Is a directory-scoped incremental scan enough?
2. **Root overlap.** The spec de-duplicates by shallower root. Is that always
   what a user expects when roots are on different volumes?
3. **mtime precision.** Some FUSE filesystems report seconds. Should the
   unchanged test include a content hash for files under 64 MB? Cost: one read
   per scan, which violates metadata-only. Current answer: no.
4. **Watcher in the GUI.** Embedding a watcher thread in a GTK process is fine,
   but the index is a single SQLite writer. Decide whether the GUI spawns the
   watcher child or owns it once tabs are live.
5. **JSON schema stability.** Freeze the scan summary schema before scripting
   against it.
