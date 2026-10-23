# Product requirements

**Status:** living document. Milestone 0 is shipped; the v1.0 scope below is
the target.

## One sentence

Folderol is a Linux-native file browser that learns where your files are and
what they are, without sending anything off your machine or moving anything
without your say-so.

## The problem

A file browser today knows one thing about a file: its path. That is enough
when you remember the path. It is useless when you remember the invoice, the
client, the colour of the cover, or the week you made it.

Three existing approaches fail in different ways:

1. **Location-only browsers** (Nautilus, Dolphin, Thunar) are fast and native,
   but search is lexical and organization is entirely manual. The user becomes
   the index.
2. **Cloud AI file tools** upload filenames, contents and behaviour to a
   service. That is unacceptable for legal, medical, research and personal
   archives, and it makes the tool unavailable offline.
3. **Auto-organizers** move files based on rules or model guesses. When the
   guess is wrong, the user has to find what the tool hid. Trust is lost on the
   first mistake.

Folderol takes the useful part of each: native speed and real file management,
local semantic understanding, and suggestions that never act on their own.

## Personas

### Mina — the researcher

- 380,000 PDFs, half of them named after download timestamps.
- Remembers authors, topics and the shape of a paper, not filenames.
- Works on a laptop, often offline, sometimes on a train.
- Needs: find by meaning, keep bibliographies together, never upload
  unpublished work.

### Dev — the developer

- A home directory full of repositories, build artefacts and dotfiles.
- Wants `target/` and `node_modules/` ignored by default, and `.gitignore`
  respected without configuration.
- Lives on the keyboard; opens the same three folders fifty times a day.
- Needs: instant navigation, command palette, no index noise, a file manager
  that does not become an Electron app.

### Imani — the creative

- External SSDs with RAW photos, footage, fonts and 3D assets.
- Thinks in thumbnails and dates, not file names.
- Needs: grid view, previews, batch rename, safe moves between drives,
  removable media that appears and disappears without breaking the app.

### Tobias — the privacy-minded sysadmin

- Will read the source before trusting it with `/home`.
- Runs an unusual compositor and expects Wayland to work.
- Needs: an auditable index, no network code, a documented deletion path, and
  packaging that does not smuggle a service in.

## Jobs to be done

| Job | Today | With Folderol |
| --- | --- | --- |
| "Find the lease I scanned last spring." | Guess folder, grep filenames. | Describe it; hybrid search ranks by meaning and words. |
| "Where did that invoice go?" | Search three clouds and Downloads. | One index; results explain why they matched. |
| "File these 40 receipts." | Drag one by one, or write a script. | Correct a few, review suggested similar files, apply once. |
| "What is eating my disk?" | `du`, then a treemap tool. | Size-aware browsing and category breakdown (1.1). |
| "Rename these 200 photos." | A shell loop. | Batch rename with preview and undo (1.0). |
| "Open the project I worked on yesterday." | Recent files in another app. | Recency-aware ranking and a recents place. |
| "Keep work out of my personal index." | Separate user accounts. | Per-root inclusion and content opt-in. |

## Product principles

1. **The filesystem is the source of truth.** Folderol never owns files. Delete
   the index and nothing is lost but memory.
2. **Native or nothing.** GTK4, Wayland, real drag and drop, real accessibility.
   No embedded browser engine.
3. **Local by default, network never.** No account, telemetry, sync or remote
   inference. Model downloads are explicit.
4. **Suggest, never act.** The model may rank, label and propose. Only the
   person moves, renames or deletes.
5. **Explainable confidence.** A wrong guess must be visible before it is
   trusted. Silence beats a confident lie.
6. **Fast is a feature.** Every interaction has a latency budget and a test.
7. **Boring about data.** GIO operations, XDG trash, an undo journal, no
   bespoke deletion code.

## v1.0 scope

### In

- Native browser: places, tabs, history, list and grid, preview inspector.
- Full local file operations: create, rename, batch rename, copy, move,
  duplicate, trash, restore, properties, open with, open terminal.
- Search: lexical + semantic, scopes, saved searches, result explanations.
- Learning: category priors, corrections, confidence, smart-folder suggestions.
- Index: ignore rules, incremental scans, live updates, multiple roots,
  exclusions, privacy controls.
- Packaging: AUR and Flatpak, `.desktop` integration, AppStream metadata.
- Accessibility: keyboard-complete, screen-reader labels, contrast, reduced
  motion.

### Out (deferred, with reasons)

| Out of scope | Why |
| --- | --- |
| Windows and macOS | GTK runs there, but packaging, trash and file semantics would double the test matrix. Linux is the promise. |
| Cloud sync or accounts | Contradicts local-first; a sync product is a different company. |
| Content editing | Folderol opens the right app; it does not become an editor. |
| Automatic filing | The first wrong move destroys trust. Suggestions only. |
| Plugin marketplace | Supply-chain and API-stability cost before 1.0. |
| Network browsing (SFTP/SMB) | Requires credential storage, mount lifecycle and a different error model. Revisit after 1.0. |
| Archive mounting | Extraction is 1.1; mounting is a filesystem in userspace, not a browser feature. |
| Root/admin operations | A file manager should not escalate privileges. |

## Differentiation

| | Native browsers | Cloud AI tools | Folderol |
| --- | --- | --- | --- |
| Native speed | yes | no | yes |
| Works offline | yes | no | yes |
| Semantic search | no | yes | yes, local |
| Learns this user | no | yes, server-side | yes, on-device |
| Explains a result | n/a | rarely | always |
| Moves files automatically | no | sometimes | never |
| Auditable index | n/a | no | SQLite you can open |

## Success metrics

Folderol collects no telemetry. These are measured through the local evaluation
harness, opt-in diagnostic exports and structured user testing.

### North star

**Successful find-or-file actions per active week.** A find-or-file action is a
search that ends in opening a result, or a correction that is accepted. It
captures the two things the product exists to do.

### Quality gates

| Metric | Target | Method |
| --- | --- | --- |
| Search success in user tests | ≥ 85% of tasks | 12-person moderated study, scripted tasks |
| Time to first correct result | ≤ 8 s median | same study |
| Category accuracy after 50 corrections | ≥ prior + 15 points | local eval corpus per participant |
| Search precision@3 on golden set | ≥ 0.80 | `TESTING.md` corpus |
| Query latency, 100k files | ≤ 30 ms p95 | `folderol bench` |
| Cold scan, 100k files | ≤ 60 s | reference NVMe machine |
| Warm startup | ≤ 400 ms | GUI stopwatch test |
| Resident memory, 100k files | ≤ 300 MB | `/proc/self/status` in CI |
| Data-loss incidents | 0 | issue tracker, undo tests |
| Crash-free sessions | ≥ 99.5% | opt-in crash reports, if ever added |

### Learning health

- Corrections per active user per week: 1–10 is healthy. Zero means the
  suggestions are invisible; hundreds means the model is bad or the UI is
  annoying.
- Suggestion acceptance rate: 40–70%. Above 90% suggests the user is not
  reading; below 20% suggests noise.
- Low-confidence share: under 25% of indexed files after 100 corrections.

## Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Confident wrong category | Trust collapse | Confidence threshold, no auto-move, one-click correction, explanation panel |
| Index leaks paths | Privacy breach | `0600` file, `0700` dir, no content by default, documented delete |
| Scan too noisy | Unusable index | Ignore files, default excludes, roots, per-root opt-in, preview counts before commit |
| Model download is huge | Abandonment | Optional download, pinned checksum, hashing fallback, size shown first |
| GTK version skew | Install failures | Flatpak as primary distribution, documented minimums |
| Filesystem edge cases | Data loss | GIO, trash, collision policy, undo journal, no symlink following |
| Scope creep | Never ships | Milestones, explicit non-goals, one-sentence UI rule |
| Semantic search feels magic | Users cannot predict it | Result reasons, scopes, ranking controls, lexical fallback |

## Assumptions and constraints

- Linux with a modern session: Wayland preferred, X11 supported. GTK 4.14+ and
  libadwaita 1.4+ for source builds; Flatpak carries its own.
- The user can read their own files; Folderol never runs as root and never asks
  for elevated privileges.
- A home directory is 10k–1M files. A single directory is usually under 10k but
  may reach 100k.
- SQLite is available embedded; no database server.
- The repository is GPL-3.0-or-later. Model weights must be permissively
  licensed and recorded in a manifest.

## Glossary

| Term | Meaning |
| --- | --- |
| Catalog / index | The SQLite database of metadata, vectors, labels and events. |
| Embedding | A 384-float vector representing a file's feature text. |
| Embedder | The component that produces vectors; hashed baseline now, ONNX later. |
| Prior | The category guess from extension/MIME before personal learning. |
| Label | An explicit human correction stored with a file. |
| Category | A model-assigned class shown in the UI. Never a folder. |
| Tag | A human-authored keyword. Arrives in M4; distinct from category. |
| Confidence | Normalised model probability shown as a percentage above 70%. |
| Smart folder | A saved query proposed from labels or co-access. Stores no files. |
| Evidence | The features that produced a category or ranking, shown in the inspector. |
| Root | A directory the user has chosen to index. |
| Scan | A walk of one or more roots that writes to the catalog. |
| Watch | A long-running process that applies filesystem events to the catalog. |

## Decision log

Product-level decisions that are not ADRs:

- 2026-10-23 — Name **Folderol**; tagline “files, with memory”.
- 2026-10-23 — Category chips appear only at ≥70% confidence.
- 2026-10-23 — Default excludes are `node_modules/`, `target/` and tool caches;
  `build/` and `dist/` are **not** excluded because they often contain
  deliverables.
- 2026-10-23 — The first release targets XDG trash, not permanent deletion.
- 2026-10-23 — Search results are grouped (best, this folder, similar) rather
  than one undifferentiated list.
