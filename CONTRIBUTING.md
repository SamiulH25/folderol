# Contributing to Folderol

Thanks for looking. Folderol is early; the most useful contributions right now
are retrieval quality, filesystem edge cases, and honest UI review.

## Prerequisites

- Rust stable (1.88+). `rust-toolchain.toml` pins the stable channel.
- GTK 4.14+ and libadwaita 1.4+ development packages, only for the GUI crate.

Arch / Omarchy:

```bash
sudo pacman -S --needed rust gtk4 libadwaita sqlite
```

Debian / Ubuntu:

```bash
sudo apt install build-essential libgtk-4-dev libadwaita-1-dev
```

SQLite is compiled in, so no system SQLite is required.

## Build and test

```bash
cargo build                     # engine + CLI (no GTK needed)
cargo test                      # unit and integration tests
cargo clippy --workspace --all-targets
cargo fmt --all
cargo build -p folderol-gui     # the shell
```

Try the engine against a small directory before touching the GUI:

```bash
cargo run -p folderol-cli -- --db /tmp/folderol-demo.sqlite3 scan ./crates
cargo run -p folderol-cli -- --db /tmp/folderol-demo.sqlite3 search "sqlite catalog"
```

## What to work on

- Issues labelled `good first issue` are scoped and have acceptance criteria.
- Retrieval changes should mention the effect on the fixture corpus. There is a
  labelled query set under `tests/corpus` (added in M1); until then, describe
  the before/after rankings in the pull request.
- UI changes should include a screenshot or a short screen recording. Wayland
  behaviour is the reference; note any X11 differences.
- New categories need a slug, a label, extension/MIME mappings, and a test in
  `folderol-core`. Slugs are persisted, so renaming one is a migration.

## Style

- Rust 2024, `rustfmt` defaults, max line width 100.
- Comments explain *why*. A comment that restates the code is deleted.
- Errors are values with context; no `unwrap()` in library code unless the
  invariant is local and proven.
- Public APIs in `core`, `ml` and `index` are allowed to change before 1.0, but
  the dependency direction in ADR 0005 does not.
- No network calls in the engine. No new dependency that phones home.
- Keep the GUI free of domain logic: it reads `Catalog`, it does not decide what
  a category means.

## Commits and pull requests

- Imperative subject, scoped: `index: survive a file moved during scan`.
- One logical change per commit. Formatting-only commits are separate.
- A pull request explains the user-visible effect, the test that proves it, and
  any trade-off made.
- Do not include generated-by or tool-attribution trailers. Write as yourself.

## Documentation

Behaviour changes update the documents that describe them, in the same pull
request:

- product scope or metrics → `docs/PRODUCT.md`
- navigation, operations or search → `docs/FILE-MANAGER.md`
- scanning, watching or permissions → `docs/M1-INDEXING.md`
- embeddings, classifier or ranking → `docs/ML.md`
- loading, empty or error states → `docs/STATES.md`
- tests or performance budgets → `docs/TESTING.md`

The index of documents and their status vocabulary is `docs/README.md`. A stale
document is a bug; if the code and the document disagree, fix one of them before
merging.

## Architecture decisions

Any choice that is expensive to reverse gets an ADR in `docs/adr/`:

1. Copy the shape of an existing record: status, date, context, decision,
   alternatives, consequences.
2. Number it sequentially; do not renumber.
3. Open it in the same pull request as the code, or earlier.
4. Superseding an ADR means marking the old one `superseded by NNNN`, not
   deleting it.

If a change contradicts an ADR, the ADR changes first.

## Licensing

Contributions are accepted under GPL-3.0-or-later. By opening a pull request you
confirm you have the right to submit the work under that license. There is no
CLA.
