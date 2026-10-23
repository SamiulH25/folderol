# Design

## The feeling

Folderol should feel like a well-made tool, not a demo of a model. Open it and
you are in your files in under half a second: dense, quiet, keyboard-first.
The intelligence is ambient. It never interrupts, never chats, and never moves
anything without being asked.

Three words guide every screen: **calm, legible, quick.**

- **Calm** — no badges begging for attention, no gradient soup, no assistant
  avatar. One accent colour, used for focus and selection.
- **Legible** — paths are text, not breadcrumbs you have to decode. Confidence
  is a number you can inspect, not a magic sparkle.
- **Quick** — every interaction has a key, every list is virtualized, every
  search is instant enough to feel like filtering rather than querying.

## Layout

```text
┌─────────────────────────────────────────────────────────────────────────┐
│  ⌘K  Search files, or describe what you remember…          [list|grid]  │
├──────────────┬──────────────────────────────────────────┬───────────────┤
│  Places      │  Home / Finance / 2024                   │  Preview      │
│   Home       │  ──────────────────────────────────────  │  [thumbnail]  │
│   Documents  │  ▸ Invoice_Acme_2024.pdf   Documents 92% │               │
│   Downloads  │    Finance · 2.1 MB · 12 Mar            │  Invoice_…    │
│              │  ▸ tax_return_2023.pdf     Documents 88% │  PDF · 2.1 MB │
│  Learned     │    Finance · 884 KB · 4 Feb             │               │
│   Documents  │  ▸ budget_2024.xlsx        Data 81%      │  Why?         │
│   Images     │    Finance · 48 KB · 9 Jan              │  name, folder │
│   Code       │                                          │  ext, age     │
│              │                                          │               │
│  Tags        │                                          │  [Open] [⋯]   │
└──────────────┴──────────────────────────────────────────┴───────────────┘
```

Widths: sidebar 240–280 px, inspector 300–360 px, content fluid. Minimum window
900×600; default 1180×760. Below 1000 px the inspector collapses into a sheet,
below 760 px the sidebar becomes an overlay.

### Sidebar

- **Places** — XDG directories, mounted volumes, pinned folders. Reorderable.
- **Learned** — categories with counts, ordered by how much they are used.
  A category with fewer than three files is folded into "More".
- **Tags** — user tags, not model output. Model categories and human tags are
  visually distinct on purpose.
- Footer: indexing status, never a spinner that hides progress.

### Content

List view is the default. Rows are 44 px (comfortable) or 32 px (compact):

```text
[icon]  Invoice_Acme_2024.pdf               Documents ●92%   2.1 MB   12 Mar
        ~/Documents/Finance/2024
```

- Icon or thumbnail, 20 px.
- Name in UI weight 600; path in muted mono at 11 px.
- Category chip appears only above 70% confidence. Below that, no chip — silence
  is more honest than a guess.
- Confidence is a dot + number, never colour alone.
- Selection uses the accent at 12% fill with a 2 px leading bar.
- Hover reveals a single `⋯` button. Everything else lives in the context menu
  and command palette.

Grid view: 168 px cards, thumbnail fills 4:3, name wraps to two lines, category
chip on hover. Grid is for images, video and 3D; list is for everything else.
The toggle remembers per-folder choice.

### Inspector

- Preview-first. Images, video posters, PDF first page, text/code with syntax
  highlighting, audio waveform. Uses glycin for sandboxed image decoding.
- Metadata: size, modified, created, MIME, dimensions/duration, owner.
- **Why this category?** expands to the features that fired: name tokens, path
  tokens, extension, and the human label history. No "AI magic" copy.
- Actions: Open, Open with, Reveal, Rename, Move to Trash. Destructive actions
  require the item to be visible and are undoable for 10 seconds via a toast.

### Command palette (Ctrl+K)

One input, three result classes:

1. **Actions** — "new folder", "toggle grid", "rescan this folder", "file under
   Documents…".
2. **Files** — the same hybrid search as the main field, top 5.
3. **Places** — jump to a path, including `~`, `/`, and `smb://` later.

Fuzzy matching with match highlighting. Enter runs the first result; Tab
completes paths. No nested modes.

### Search

The main search field is the product's front door. It accepts:

- words: `acme invoice`
- natural language: `the lease for the flat`
- scopes: `type:pdf`, `in:Finance`, `before:2024-01-01`, `category:documents`
- quoted phrases: `"Q3 report"`

Results are grouped, not just ranked: **Best matches**, **In this folder**,
**Similar files**. Under each result, a one-line reason (`matched name and
folder`, `same words, nearby in meaning`). Advanced ranking controls live
behind a small "Ranking" popover, with the weights exposed as sliders for people
who care. Defaults are good; the knobs are for trust.

Natural-language queries are embedded on the fly. On a cold model load the
field shows "warming up the local model" once, for under a second, not a
blocking modal.

### Smart folders

Generated from clusters of labels and folder co-access, proposed rather than
created by default: "Invoices — 34 files across Finance and Downloads". Pinning
one turns it into a saved query, visible in the sidebar. Smart folders never
own files; deleting one deletes the query, not the contents.

### Learning moments

- Correcting a category is a two-click action: chip → pick category, or drag
  the row onto a category in the sidebar.
- After a correction, a toast: "Filed under Documents. Apply to 11 similar
  files?" with Apply / Review / Dismiss. Review opens a multi-select list.
- A weekly (opt-in) "Tidy suggestions" sheet lists low-confidence files and
  duplicate names. No modal on startup, ever.
- A small "Learning" panel in Settings shows label counts per category and a
  reset button that deletes `model.json` without touching files.

## Visual system

Dark first, light available. The palette is warm-neutral, not blue-grey, so
thumbnails carry the colour.

| Token | Dark | Light | Use |
| --- | --- | --- | --- |
| `bg` | `#0e1013` | `#f6f5f2` | window |
| `surface` | `#15181d` | `#ffffff` | rows, inspector |
| `surface-hover` | `#1b1f26` | `#efede8` | hover |
| `text` | `#e8eaed` | `#1b1d21` | primary |
| `text-muted` | `#9aa1ab` | `#6b7078` | paths, metadata |
| `accent` | `#f2b544` | `#b4780a` | selection, focus |
| `success` | `#4fd1c5` | `#0f766e` | confirmed labels |
| `danger` | `#f87171` | `#b91c1c` | destructive only |

- **Type**: Inter (UI), JetBrains Mono (paths, sizes, hashes). Fall back to
  system Cantarell / monospace. Never below 11 px; paths at 11 px mono.
- **Spacing**: 4 / 8 / 12 / 16 / 24. Row padding 12 px.
- **Radius**: 8 px controls, 12 px rows, 16 px panels.
- **Elevation**: one shadow level. Panels are separated by a 1 px border at 8%
  white, not by stacking shadows.
- **Motion**: 120–200 ms, `cubic-bezier(0.2, 0.8, 0.2, 1)`. Lists fade+slide 4
  px, inspector slides 12 px, toasts rise 8 px. No springs, no bounce. Reduced
  motion collapses everything to a 100 ms opacity change.
- **Icons**: symbolic, 16/20 px, one family (Adwaita). File type icons only in
  list view when no thumbnail exists.
- **Texture**: none. No noise overlays, no glassmorphism. The only translucency
  is the sidebar on compositors that support blur, and it is optional.

## Keyboard

| Key | Action |
| --- | --- |
| `Ctrl+K` | command palette |
| `/` or `Ctrl+F` | focus search |
| `j` / `k` | next / previous row |
| `Enter` | open |
| `Space` | quick look |
| `Ctrl+Shift+N` | new folder |
| `F2` | rename |
| `Delete` | move to trash (undo toast) |
| `Ctrl+L` | edit path |
| `Alt+Left/Right` | back / forward |
| `Ctrl+1/2` | list / grid |
| `Ctrl+I` | toggle inspector |
| `?` | shortcut sheet |

Keys never fire while a text field has focus. Every pointer action has a key
equivalent.

## Accessibility

- Contrast ≥ 4.5:1 for text, ≥ 3:1 for focus indicators.
- Category is never conveyed by colour alone; the chip carries a label.
- Full keyboard traversal, visible focus ring using the accent with a 2 px
  offset.
- Screen reader names for rows include name, category and confidence.
- `prefers-reduced-motion` respected via GTK settings.
- Hit targets ≥ 32 px; row actions also available from the context menu.

## Performance as design

- The first paint never waits for the catalog: show Places, then stream rows.
- Debounce search at 60 ms, cancel superseded queries, cap at 80 rows.
- Thumbnails generate off the main thread with a bounded queue and a disk cache;
  list view uses icons only until a thumbnail is ready.
- Directory changes update the affected rows, never reload the view.
- No animation runs while a directory with > 2,000 entries is loading.

## Anti-goals

- A chat pane. Search is not a conversation.
- Auto-filing, auto-tagging without consent, or any silent mutation.
- A web view, Electron, or a bundled browser engine.
- Telemetry, crash uploads, accounts, or a cloud index.
- Clever empty states that hide the filesystem. An empty folder says it is
  empty and offers to create something.
