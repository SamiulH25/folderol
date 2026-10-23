# Folderol

[![CI](https://github.com/SamiulH25/folderol/actions/workflows/ci.yml/badge.svg)](https://github.com/SamiulH25/folderol/actions/workflows/ci.yml)

**A Linux-native file browser that learns where things belong.**

Folderol indexes your files locally, embeds what they are and where they live, and learns from the way *you* file things. It is a real file browser first — fast, keyboard-driven, native GTK4 — and a semantic one second.

> **Status: pre-alpha, milestone 0.**
> The catalog, hybrid search, online classifier and CLI work today. The GTK shell is a first slice, not yet a daily driver. See [ROADMAP.md](ROADMAP.md).

```console
$ folderol scan ~/Documents ~/Pictures
Indexed 4812 files into ~/.local/share/folderol/index.sqlite3 (4812 total).

$ folderol search "invoice from acme"
SCORE  CATEGORY     MEANING PATH
1.528  Documents    0.44    ~/Documents/Finance/Invoice_Acme_2024.pdf

$ folderol learn ~/Documents/Finance/Invoice_Acme_2024.pdf --category documents
Learned Documents for Invoice_Acme_2024.pdf. 14 labels, 14 training samples,
loss 0.081, 96 files reclassified.
```

## Why this exists

Every file browser knows exactly one thing about your files: where they are. None of them know *what* the files are, or that the invoice from Acme lives next to the tax return because that is where you always put paperwork.

Folderol closes that gap without sending a byte off your machine:

- **Local embeddings** turn a file's name, path, type and shape into a vector.
- **A small classifier learns from your corrections** — not from a vendor's idea of what a "document" is.
- **Hybrid retrieval** blends exact words, meaning, category confidence and recency, and shows you why each result ranked where it did.
- **Nothing is ever moved automatically.** Folderol suggests; you decide.

## Design principles

1. **Native, not a web page.** GTK4 + libadwaita, real Wayland clients, real drag and drop, real accessibility.
2. **Local-first.** No account, no telemetry, no network calls. Model downloads are explicit and checksummed.
3. **Learn from behaviour, not just extensions.** A `.pdf` can be an invoice, a paper, or a lease.
4. **Explainable.** Every category and every ranking can be traced to features you can read.
5. **Boring about your data.** The index is one SQLite file you can inspect, back up or delete. Destructive operations go through GIO trash, never bespoke `rm` logic.

## Stack

| Layer | Choice |
| --- | --- |
| Language | Rust 2024 |
| UI | GTK 4.22 + libadwaita 1.9 (`gtk4-rs`, custom CSS) |
| Catalog | SQLite 3 with FTS5, one file, WAL mode |
| Embeddings | `Embedder` trait: hashing baseline now, ONNX MiniLM next |
| Classifier | One-vs-rest logistic regression trained on your labels |
| Targets | Linux (Wayland first, X11 works) |

## Build

Prerequisites on Arch / Omarchy:

```bash
sudo pacman -S --needed rust gtk4 libadwaita sqlite
```

Then:

```bash
cargo build --release            # headless engine + CLI
cargo build --release -p folderol-gui
```

The GUI is excluded from the default workspace build so servers and CI can use
the engine without GTK headers installed.

Run the pieces:

```bash
cargo run -p folderol-cli -- scan ~/Documents
cargo run -p folderol-cli -- search "last quarter budget"
cargo run -p folderol-cli -- stats
cargo run -p folderol-gui
```

Override the catalog location with `--db PATH` or `FOLDEROL_DB`. Default:
`$XDG_DATA_HOME/folderol/index.sqlite3`.

## Repository layout

```
crates/
  folderol-core/    domain types, tokenization, feature text, category priors
  folderol-ml/      embedder trait, hashing embedder, online classifier
  folderol-index/   SQLite catalog, FTS5, hybrid retrieval
  folderol-scan/    ignore-aware walking, batching, watch mode (planned, M1)
  folderol-cli/     scan / watch / search / learn / stats / bench
  folderol-gui/     GTK4 + libadwaita shell
docs/               product, behaviour, indexing, ML, testing, release specs
docs/adr/           architecture decisions, one per irreversible choice
```

Dependency direction is strict: `core ← ml ← index ← {cli, gui}`. No GTK type
ever leaks into the engine, and no engine crate opens a socket.

## Documentation

Start with the [documentation index](docs/README.md).

- [docs/PRODUCT.md](docs/PRODUCT.md) — problem, personas, scope, success metrics
- [docs/FILE-MANAGER.md](docs/FILE-MANAGER.md) — the complete browser and file-operation spec
- [DESIGN.md](DESIGN.md) — interaction and visual system
- [ARCHITECTURE.md](ARCHITECTURE.md) — how indexing, retrieval and learning fit together
- [docs/M1-INDEXING.md](docs/M1-INDEXING.md) — the trustworthy-index specification
- [docs/ML.md](docs/ML.md) — features, embeddings, classifier, evaluation
- [docs/STATES.md](docs/STATES.md) — loading, empty, error and confirmation copy
- [docs/TESTING.md](docs/TESTING.md) — correctness, retrieval and performance proof
- [docs/RELEASING.md](docs/RELEASING.md) — packaging and release process
- [ROADMAP.md](ROADMAP.md) — milestones and non-goals
- [docs/adr/](docs/adr/) — why the stack looks like this
- [CONTRIBUTING.md](CONTRIBUTING.md) — build, test, and decision process

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
