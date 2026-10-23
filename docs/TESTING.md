# Testing strategy

**Status:** M0 tests exist; this document defines the target harness through
v1.0.

## 1. What “tested” means here

A file manager can destroy data. A search regression is annoying; a move bug is
unacceptable. The test strategy is weighted accordingly:

1. **Correctness of operations** — highest priority, with adversarial fixtures.
2. **Index correctness** — counts, deletions, renames, labels, migrations.
3. **Retrieval quality** — measured, not vibes.
4. **Responsiveness** — latency budgets with a benchmark, not anecdotes.
5. **Visual and interaction polish** — manual matrix plus screenshots.

Coverage percentage is not a gate. Critical modules are expected to be near
100% by branch, but the gate is behaviour: every requirement in
`FILE-MANAGER.md` has a test or a documented manual check.

## 2. Reference environment

| | |
| --- | --- |
| CPU | 8-core x86_64, 3.2 GHz |
| RAM | 32 GB |
| Disk | NVMe, ≥ 2 GB/s sequential |
| OS | Arch Linux, current |
| Session | Wayland (Hyprland), X11 fallback |
| Rust | stable from `rust-toolchain.toml` |

CI runs Ubuntu 24.04 with GTK 4.14 and libadwaita 1.5. A second job runs the
engine inside `archlinux:latest` to catch packaging drift.

## 3. Layers

| Layer | Location | Runs in CI | Purpose |
| --- | --- | --- | --- |
| Unit | `#[cfg(test)]` next to code | yes | pure functions, schema, ranking gates |
| Integration | `crates/*/tests/` | yes | scan → catalog → search with temp trees |
| Golden retrieval | `tests/ranking/` | yes | precision@3, MRR, regression gate |
| Watcher | `crates/folderol-scan/tests/watch.rs` | yes | event mapping with real inotify |
| CLI contract | `crates/folderol-cli/tests/` | yes | exit codes, stdout/stderr separation |
| GUI smoke | `crates/folderol-gui/tests/` under Broadway | yes | window builds, rows render, no panic |
| Performance | `folderol bench` | yes (generous) | latency trend |
| Manual matrix | `docs/TESTING.md` §8 | release | what automation cannot see |

## 4. Unit tests

Required coverage by module:

### `folderol-core`

- tokenizer: camelCase, snake_case, kebab-case, Unicode, CJK, emoji, combining
  marks, long tokens, numeric tokens.
- feature text: every part present, no content read, stable order.
- category prior: every extension in the table, MIME fallback, unknown types,
  directories.
- path/size bucket boundaries (0, 1, 4095, 4096, …).

### `folderol-ml`

- embedding determinism, L2 norm, zero vector for empty text.
- similarity ordering for related vs unrelated text.
- classifier convergence on separable data.
- dimension mismatch is an error, not a zero score.
- probabilities sum to one.
- JSON round-trip and corrupt-model rejection.
- fewer than 3 labels does not reclassify (CLI-level test).

### `folderol-index`

- schema migration from 1 to 2 and idempotent reopen.
- upsert is idempotent; size/mtime updates.
- label survives a rescan with a different prior.
- `is_current` for every path/size/mtime combination, including `NULL` mtime.
- `remove_tree` matches exact and descendants, not a shared prefix.
- FTS row deleted with the file; no orphan rows.
- `meta` round-trip and unknown keys preserved.
- DB file mode `0600`.
- corrupt embedding blob returns an error, not a panic.
- search: lexical hit, semantic hit, nonsense query returns nothing, weights
  change the order, limit respected.

### `folderol-scan`

- default excludes, hidden, `.folderolignore`, `.gitignore` inside/outside a
  repository.
- symlinks skipped by default, followed only with the flag, never escape root.
- cross-filesystem default and opt-in.
- unchanged/skip logic.
- cancellation flushes the batch and does not advance meta.
- error on one file does not abort the scan.
- DB path excluded from its own scan.

## 5. Fixture corpus

`tests/fixtures/` is committed and contains **empty files** with meaningful
names plus generated content where a test needs bytes.

```text
tests/fixtures/home/
  Finance/2024/Invoice_Acme_2024.pdf
  Finance/2024/tax_return_2023.pdf
  Finance/receipts/receipt-001.txt
  Projects/folderol/src/main.rs
  Projects/folderol/Cargo.toml
  Pictures/2026/beach-sunset.jpg
  Pictures/2026/screenshot-2026-10-23.png
  Documents/Mietvertrag_Wohnung.pdf
  Downloads/report (1).pdf
  Downloads/report (2).pdf
  Music/album/01 - Intro.flac
  .hidden/secret.txt
  node_modules/pkg/index.js
  target/debug/folderol
  .folderolignore
```

Rules:

- No real personal data, ever.
- File sizes are mostly zero; large-file tests generate sparse files at runtime.
- Binary fixtures are avoided when a text file exercises the same path.
- The corpus is versioned with the tests that judge it.

## 6. Golden retrieval

`tests/ranking/cases.toml`:

```toml
[[case]]
query = "invoice acme"
acceptable = ["Finance/2024/Invoice_Acme_2024.pdf"]
notes = "exact name tokens"

[[case]]
query = "the lease for the flat"
acceptable = ["Documents/Mietvertrag_Wohnung.pdf"]
requires = "semantic"   # skipped for hashing baseline, required for ONNX

[[case]]
query = "zzzzqqqq"
acceptable = []
notes = "negative; no lexical or semantic hit"
```

The runner builds a temporary in-memory catalog from the fixture corpus and
computes precision@3, MRR@10 and recall@20. A change that drops any metric by
more than 3 points fails CI. Results are printed as a table and stored as a CI
artifact.

Retrieval changes must include:

- the before/after table,
- the queries that changed,
- an explanation of why the change is correct, not just better on the suite.

## 7. Watcher tests

Real inotify against `tempfile::tempdir()`:

| Scenario | Expected |
| --- | --- |
| create file | row appears within 1 s |
| write file | size/vector updated |
| rename file | old path gone, new path present |
| delete file | row and FTS removed |
| create directory with files | subtree indexed |
| rename directory | old subtree gone, new subtree indexed |
| delete directory | descendants removed |
| create inside ignored dir | not indexed |
| edit `.folderolignore` | containing tree rescanned |
| burst of 500 writes | one batch, no duplicate rows |
| DB inside watched root | DB never indexed |
| watcher stopped | no further rows after shutdown |

Timing tests are inherently flaky. Use a 5 s timeout with polling at 50 ms, not
a fixed sleep. Retry once before failing, and mark the test `#[ignore]` if it
runs on a filesystem without inotify.

## 8. Manual GUI matrix

Run before every minor release:

| Area | Cases |
| --- | --- |
| Wayland | launch, DnD in/out, portals, fractional scaling 125%/150%, multi-monitor |
| X11 | launch, DnD, focus, window resize |
| HiDPI | 1×, 2×, fractional; no clipped text at 200% font scale |
| RTL | Arabic locale; sidebar and columns mirror; paths do not |
| Keyboard | every action in the palette reachable without a pointer |
| Screen reader | Orca: list, search, operation progress, error dialog |
| Reduced motion | no slide/scale; fades only |
| High contrast | borders visible; category not colour-only |
| 100k directory | scroll, sort, search-in-folder, no freeze > 200 ms |
| Removable drive | mount, browse, copy, yank mid-copy, remount |
| Trash | trash, restore, collision, empty, external volume unavailable |
| Operations | copy 10 GB, cancel, resume, error report, undo |
| Crash | kill -9 during move; relaunch journal recovery |

Screenshots for the release notes are captured at 2× and stored under
`docs/media/` (not committed until the UI stabilises).

## 9. Performance harness

```bash
cargo run --release -p folderol-cli -- bench \
  --files 100000 --queries 20 --assert-search-ms 30
```

Measures:

- index build time (embedding + SQLite),
- query p50/p95/max,
- peak RSS (read from `/proc/self/status` on Linux),
- index file size.

CI runs 20k files with a 150 ms gate to avoid shared-runner noise. The 100k
numbers are recorded manually in `docs/benchmarks/`. A release cannot ship with
a p95 more than 20% worse than the previous record without an explanation.

Scan benchmark: generate 100k files with realistic names in a temp tree, run
`folderol scan`, record wall time, files/second, peak RSS, DB size, and the
second-scan time. Script lives in `scripts/gen-corpus.sh` (planned).

## 10. Security and robustness tests

- Malformed/corrupt SQLite file: error, no panic, browsing still possible.
- Truncated embedding BLOB: error, row skipped.
- FTS query injection: quotes, `*`, `NEAR`, parentheses in user input are
  tokenized, never interpreted.
- Path traversal: `../` in a name field cannot escape the operation root.
- Symlink race: a file replaced by a symlink between stat and open is handled by
  GIO and never followed for writes.
- Permission-denied subtree during scan: counted, not fatal.
- Read-only filesystem: mutating actions disabled.
- Very long paths near `PATH_MAX`: clear validation error.
- 10k files in one directory: no O(n²) blowup.
- Fuzz target (planned): ignore-pattern matcher and tokenizer with arbitrary
  UTF-8.

## 11. CI

```yaml
jobs:
  test:        fmt --check, clippy -D warnings, test --workspace, release build
  arch:        same engine tests inside archlinux:latest
  ranking:     golden retrieval, artifact with metrics
  bench:       release bench, 20k files, 150 ms gate
  packaging:   build .desktop/metainfo, validate with appstreamcli, flatpak build
```

Required checks on `main`: `test`, `ranking`. `bench` is required once the
harness is stable; `packaging` is required for release tags.

## 12. Flaky tests

- A flaky test is a bug. It is fixed or deleted within one week, never retried
  forever in CI.
- Timing tests use polling with deadlines, not sleeps.
- Filesystem tests use `tempfile`, never `/tmp` shared paths or the repository.
- Tests must pass with `--test-threads=1` and in parallel.
- Network is never required. Model-dependent tests are `#[ignore]` and run in a
  separate job with a cached fixture model.

## 13. Release smoke

On a clean VM, for each channel:

- [ ] Install from package; launch from application menu.
- [ ] Open Home, create a folder, rename it, trash it, restore it.
- [ ] Search a known file; open it.
- [ ] Scan a small tree; verify counts.
- [ ] Delete the index; verify browsing and no file loss.
- [ ] Uninstall; verify no user files were removed and the index remains only if
      the user chose to keep it.

## 14. Test data hygiene

- Never commit a real home directory, even zipped.
- Benchmarks generate data at runtime and delete it.
- Crash dumps and diagnostic exports are opt-in and scrubbed of paths before
  sharing.
- CI caches contain only build artifacts, never user-indexed databases.
