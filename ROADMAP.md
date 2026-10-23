# Roadmap

Milestones are vertical slices: each one ends with something a person can use,
not a layer that only compiles.

## M0 — Bootstrap ✅

- [x] Rust workspace, strict crate boundaries
- [x] SQLite catalog with FTS5 and migrations
- [x] Feature extraction and hashing embedder baseline
- [x] Online logistic-regression classifier with JSON persistence
- [x] Hybrid search (words + meaning + prior + recency)
- [x] CLI: `scan`, `search`, `learn`, `stats`, `categories`
- [x] GTK4/libadwaita shell reading the same catalog
- [x] Architecture, design and decision records

**Exit criteria:** `folderol scan` then `folderol search` returns the right file
on a real home directory, and the GUI opens the catalog.

## M1 — A trustworthy index

- [ ] Ignore rules: `.gitignore`, `.folderolignore`, XDG trash, build dirs
- [ ] Include/exclude roots, per-root content opt-in
- [ ] Progress reporting, cancellation, resumable scans
- [ ] Incremental updates via inotify (debounced, move-aware)
- [x] Store `embedder_id` + `dim` in `meta`; warn on mismatch
- [ ] Automatic vector rebuild on model change
- [ ] Index permissions `0600`, migration tests, backup/restore docs
- [ ] Benchmarks: 10k / 100k / 1M synthetic files, CI regression gate
- [ ] Packaging: AUR `folderol-git`, Flatpak manifest

**Exit criteria:** point it at `$HOME`, interrupt it, restart it, and the counts
are correct. A file renamed on disk is findable by its new name within a second.

## M2 — Real semantics

- [ ] `ort` + ONNX `all-MiniLM-L6-v2` (quantized) behind the existing trait
- [ ] Explicit model download with pinned SHA-256 and offline fallback
- [ ] `sqlite-vec` for ANN search; keep brute force as reference
- [ ] Content extraction (text, PDF, Office) behind opt-in, chunked, capped
- [ ] Query scopes and a ranking evaluation corpus
- [ ] Category suggestions from clustering, not just classification
- [ ] "Why this result?" panel fed by real feature attributions

**Exit criteria:** "the lease for the flat" finds `Mietvertrag_Wohnung.pdf`
without the word "lease" appearing anywhere. Precision@3 on the eval set beats
the hashing baseline by a documented margin.

## M3 — The browser

- [ ] Real file view: list + grid, virtualized, sortable, resizable columns
- [ ] Tabs, back/forward history, breadcrumbs, editable path bar
- [ ] Thumbnails via glycin, disk cache, bounded worker pool
- [ ] Drag and drop: internal move/copy, external targets, spring-loaded folders
- [ ] Context menus and full keyboard map from `DESIGN.md`
- [ ] File operations through GIO: rename, move, copy, trash, undo toast
- [ ] Command palette, quick look, inspector with preview
- [ ] Volume monitoring, mount/unmount, removable media

**Exit criteria:** replaces the user's daily file manager for a week without a
data-loss incident or a "where is that file" moment.

## M4 — The learning loop

- [ ] Interaction events: open, reveal, move, co-access
- [ ] Folder affinity and "usually together" ranking
- [ ] Smart folders proposed from clusters, pinned as saved queries
- [ ] Correction flow with "apply to N similar" review
- [ ] Active learning: surface low-confidence files when the user is idle
- [ ] Explanation UI backed by classifier weights
- [ ] Evaluation harness for personalisation, with a reset switch

**Exit criteria:** after 50 corrections, the user's top category prediction is
right more often than the extension prior, measured locally.

## M5 — Polish and distribution

- [ ] Accessibility audit (Orca, keyboard-only, contrast)
- [ ] 1M-file performance pass, memory ceiling documented
- [ ] Light theme, high-contrast theme, RTL layouts
- [ ] Localisation scaffolding (gettext)
- [ ] Flatpak on Flathub, AUR binary package
- [ ] Website with a 30-second demo, no tracking
- [ ] Contribution guide for model evaluation and UI review

## Non-goals

- Windows and macOS ports. GTK4 runs there; Folderol will not be tested there.
- A sync service, cloud drive, or account system.
- Editing file contents. Folderol opens files in the right app, it does not
  become that app.
- An LLM chat interface. If a language model lands, it answers one question at
  a time inside search, with citations to files.
- Plugin marketplace. Extensions, if ever, are local directories of Rust crates.

## How work is chosen

1. Data-loss or corruption bugs, always.
2. Retrieval quality regressions on the eval corpus.
3. Anything blocking daily use.
4. Performance budgets in `ARCHITECTURE.md`.
5. New surfaces.

A milestone item that cannot be finished in a week is split. A feature that
cannot be explained in one sentence in the UI is redesigned.
