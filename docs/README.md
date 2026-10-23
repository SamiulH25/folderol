# Folderol documentation

This directory is the written contract for the product. Code is held to it; when
the two disagree, either the code is wrong or the document changes in the same
pull request.

## Reading order

| # | Document | Answers | Audience |
| --- | --- | --- | --- |
| 1 | [PRODUCT.md](PRODUCT.md) | Why does this exist, for whom, and what counts as success? | everyone |
| 2 | [FILE-MANAGER.md](FILE-MANAGER.md) | What does the application do, screen by screen and operation by operation? | design, engineering, QA |
| 3 | [DESIGN.md](../DESIGN.md) | What does it look and feel like? | design, frontend |
| 4 | [ARCHITECTURE.md](../ARCHITECTURE.md) | How is it built, and where does data flow? | engineering |
| 5 | [M1-INDEXING.md](M1-INDEXING.md) | Exactly how the trustworthy index is built. | engineering |
| 6 | [ML.md](ML.md) | How the learning and retrieval models work. | engineering, research |
| 7 | [STATES.md](STATES.md) | What every loading, empty and error state says and does. | design, frontend, QA |
| 8 | [TESTING.md](TESTING.md) | How correctness, performance and regressions are proven. | engineering, QA |
| 9 | [RELEASING.md](RELEASING.md) | How a version becomes installable software. | maintainers |
| 10 | [SECURITY.md](../SECURITY.md) | Trust boundaries, privacy and reporting. | everyone |
| 11 | [adr/](adr/) | Why irreversible choices were made, one record each. | engineering |

`ROADMAP.md` at the repository root sequences the work. `DESIGN.md` and
`ARCHITECTURE.md` also live at the root because they are the two documents most
people look for first.

## Status vocabulary

The documentation describes the **target**, not only what is merged. Every
document uses these words deliberately:

- **Shipped** — implemented on `main` and covered by tests.
- **Planned** — specified here, not built yet; the milestone is named.
- **Deferred** — deliberately out of scope for 1.0; the reason is recorded.
- **Open** — a decision still needs an ADR.

The current shipped surface is milestone 0: the catalog, hybrid search, the
online classifier, the CLI and a minimal catalog-backed GTK window. Directory
browsing, file operations, tabs and the rest of `FILE-MANAGER.md` are planned.

## Conventions

- `MUST`, `SHOULD`, `MAY` follow RFC 2119.
- Paths are shown relative to the repository unless absolute XDG paths matter.
- "Index" means the SQLite catalog, never the filesystem.
- "Category" is model-assigned. "Tag" is human-authored. They are never mixed.
- Latency figures are p95 on the reference machine described in
  `TESTING.md`, unless stated otherwise.
- A change to behaviour updates this documentation in the same pull request.

## Documentation debt

Known gaps, tracked rather than hidden:

- No screenshots or motion captures yet; `DESIGN.md` is prose and tokens.
- The settings schema is described but not implemented; its versioning story is prose, not code.
- Archive handling and network locations are deferred without a design.
- Accessibility has a checklist, not an audit report.
- Localisation is planned for M5; copy in `STATES.md` is English-only.
