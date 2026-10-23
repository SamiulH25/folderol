# Architecture

Folderol is a local-first desktop application. There is no server, no daemon
required for correctness, and no network dependency. This document describes
the parts, the data that flows between them, and the budgets each part must
respect.

## The shape of the system

```
                    ┌──────────────────────────────┐
   filesystem ─────▶│ scanner (walkdir + filters)  │
                    └──────────────┬───────────────┘
                                   │ FileRecord
                                   ▼
                    ┌──────────────────────────────┐
                    │ features (name, path, kind,  │
                    │ size, age, mime)             │
                    └──────────────┬───────────────┘
                                   │ feature text
                                   ▼
                    ┌──────────────────────────────┐
                    │ Embedder                     │
                    │  v0: hashing (384-d)         │
                    │  v1: ONNX MiniLM (384-d)     │
                    └──────────────┬───────────────┘
                                   │ vector
                                   ▼
   ┌──────────────┐    ┌──────────────────────────────┐
   │ Classifier   │───▶│ Catalog (SQLite)             │
   │ learned from │    │  files · vectors · FTS5      │
   │ user labels  │    │  labels · events             │
   └──────▲───────┘    └──────────────┬───────────────┘
          │                           │
          │ corrections               │ hybrid query
          │                           ▼
   ┌──────┴───────────────────────────────────────────┐
   │ GTK4 shell / CLI                                 │
   │ list · grid · search · inspector · explain       │
   └──────────────────────────────────────────────────┘
```

## Crates

### `folderol-core`

Domain types with no UI and almost no I/O.

- `FileRecord` — path, name, extension, MIME, size, mtime, kind. Stat-only;
  never reads contents.
- `Category` — the seeded taxonomy plus an extension/MIME prior with a
  confidence. The taxonomy is a starting point, not a constraint.
- `features::tokenize` — Unicode-aware, splits `snake_case`, `kebab-case`,
  `camelCase` and paths.
- `features::feature_text` — the exact string handed to the embedder. The
  basename appears twice because it carries more intent than the directory.

### `folderol-ml`

- `Embedder` — deterministic `embed(text) -> Vec<f32>`, a `dim`, a stable
  `model_id`, and a `semantic_floor` used to reject unsupported semantic hits.
- `HashingEmbedder` — signed feature hashing with `ln(1+tf)` compression and L2
  normalization. Dependency-free, offline, good enough to prove the pipeline.
  It is explicitly a baseline, not the product.
- `Classifier` — one-vs-rest logistic regression. A handful of floats per
  category, JSON on disk, retrainable in milliseconds. Every prediction can be
  explained by the weights that fired.

### `folderol-index`

- One SQLite file, WAL mode, `PRAGMA user_version` migrations.
- `files` holds metadata, predicted category, confidence and the vector as a
  little-endian `f32` BLOB.
- `files_fts` is an FTS5 table over name, path and feature tokens.
- `labels` stores explicit human corrections (one per file; latest wins).
- `events` is reserved for interaction signals (open, reveal, drag, co-access).
- `Catalog::search` performs hybrid retrieval in one pass and returns every
  score component for the UI to explain.

### `folderol-cli`

The reference client and the test harness for the engine. `scan`, `search`,
`learn`, `stats`, `categories`. It is the fastest way to check a retrieval
change without a display server.

### `folderol-gui`

GTK4 + libadwaita. Reads the same catalog, owns no domain logic, and does no
filesystem mutation except through GIO (milestone 3). The current shell is a
thin slice; `DESIGN.md` is the target.

## Catalog schema

```sql
files(
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  ext TEXT,
  mime TEXT,
  size INTEGER NOT NULL,
  modified INTEGER,              -- unix seconds
  category TEXT NOT NULL,        -- slug, never a display name
  confidence REAL NOT NULL,      -- 0..1
  embedding BLOB,                -- f32 little-endian, dim recorded in meta
  indexed_at INTEGER NOT NULL
)

files_fts USING fts5(name, path, tokens, tokenize='unicode61 remove_diacritics 2')

labels(file_id PRIMARY KEY, category, created_at)
events(id, file_id, kind, at)
meta(key PRIMARY KEY, value)          -- embedder_id, embedding_dim, schema notes
```

Design notes:

- **Paths are the identity.** Inode identity is unreliable across moves and
  network filesystems; a move is an update keyed by path, with the old row
  deleted by the watcher.
- **The vector is opaque.** Only the model that wrote it may read it. `meta`
  records `embedder_id` and `embedding_dim`; a mismatch triggers a background rebuild
  rather than silent nonsense.
- **No content column.** Content extraction arrives as a separate,
  opt-in stage with its own table, so the metadata index stays fast and small.

## Retrieval

```text
score = w_semantic · cosine(query, file)
      + w_lexical  · rank_score(FTS5)
      + w_prior    · category_confidence
      + w_recency  · 1 / (1 + age_days / 90)
```

Defaults: `1.0, 0.9, 0.15, 0.05`. A hit is kept only if it has lexical support
or clears the embedder's `semantic_floor`. Lexical scores are rank-normalized
(`1 / (1 + 0.25·rank)`) because raw BM25 values are not comparable across
queries.

Why not pure vector search? Names and paths are exact identifiers. A search for
`README` or `2024-Q3` must not depend on a model deciding those tokens are
semantically close. Why not pure FTS? Because "the lease for the flat" should
find `Mietvertrag_Wohnung.pdf`.

The current implementation scans all vectors in memory per query. At 100k files
× 384 dimensions that is ~38 MB and low tens of milliseconds, which is within
budget for v0. Milestone 2 moves this to `sqlite-vec` without changing the
schema shape or the scoring interface.

## Learning

Two loops, deliberately separated:

1. **Priors (cold start).** Extension and MIME map to a category with a
   confidence around 0.9 for known types and 0.25 otherwise. This is what makes
   the first launch useful.
2. **Personal model.** When the user corrects a category, the label is stored
   and the classifier is retrained from *all* labels. Corrections are strong
   signals; opens and co-access are weak signals and arrive later in `events`.

The predicted category stored in `files.category` is the classifier's answer
blended with the prior: when the classifier is uncertain (few labels, low
confidence), the prior wins. The UI always shows confidence and can always
explain a prediction.

Deliberate limits:

- No auto-moving. Folderol may suggest "12 files look like receipts — file them
  under Finance?", but the user confirms.
- No cross-user training. There is no model upload path at all.
- Removing a label is supported because training always restarts from the label
  table, not from the previous model.

## Privacy and security

- **No network code in the engine.** A model download, when it lands, is a
  separate explicit action with a pinned SHA-256.
- **The index is sensitive.** It contains full paths. Created `0600` inside
  `$XDG_DATA_HOME/folderol`. `folderol panic` (planned) deletes it.
- **No content by default.** Metadata-only scanning means no secrets are copied
  into the index. Content extraction is opt-in per root and stored separately.
- **Symlinks are not followed** during scans, avoiding cycles and surprise
  traversal outside a chosen root.
- **Destructive operations** use the XDG trash specification through GIO. There
  is no bespoke deletion code path.

## Performance budgets

| Operation | Target | Notes |
| --- | --- | --- |
| Cold scan, 100k files | < 60 s | metadata + hashing embed, NVMe |
| Incremental update | < 50 ms/file | batched in 512-row transactions |
| Hybrid query, 100k files | < 30 ms p95 | brute-force cosine, single thread |
| GUI startup (warm) | < 400 ms | catalog opens lazily, first page only |
| Memory, 100k indexed | < 300 MB | vectors + GTK resident |

Benchmarks are added in milestone 1 and run in CI on a synthetic corpus so
regressions are visible rather than felt.

## Testing strategy

- **Unit tests per crate** — tokenization, priors, embedding normalization,
  classifier convergence, schema round-trips, ranking gates.
- **Fixture corpus** — a checked-in directory of empty files with realistic
  names and types, used for golden retrieval tests.
- **Ranking evaluation** — a small labelled query set with expected top-3
  results; precision@3 must not regress when weights or models change.
- **UI smoke** — GTK under Broadway/headless in CI, plus manual Wayland checks
  for drag and drop, which no headless harness covers honestly.

## Open questions

- Whether to keep `events` in the same SQLite file or a separate append-only
  log once interaction capture lands.
- Whether thumbnails belong in the catalog or a content-addressed cache
  directory (current lean: cache directory, keyed by path + mtime + size).
- Whether the classifier should become multinomial once label counts justify
  it; one-vs-rest is easier to explain and adequate below ~10k labels.
