# File manager specification

**Status:** target specification for v1.0. Milestone 0 ships a minimal shell;
everything marked *planned* is contracted here before implementation.

Requirement IDs are stable. A test or pull request can cite `FM-NAV-04` and mean
exactly one thing. New requirements take the next free number; they are never
renumbered.

## 1. Application model

| ID | Requirement | Status |
| --- | --- | --- |
| FM-APP-01 | Folderol is a single-instance GTK application. Opening a path from the shell or the file-manager registration hands it to the running instance. | planned |
| FM-APP-02 | `folderol-gui [PATH]` opens a window at `PATH`, or the last session location when omitted. | planned |
| FM-APP-03 | A second invocation with `--new-window` opens an independent window. | planned |
| FM-APP-04 | Closing the last window quits the process; closing a window with running operations asks first. | planned |
| FM-APP-05 | Session state (window size, open tabs, current folders, view mode, hidden toggle) is restored on launch unless disabled. | planned |
| FM-APP-06 | The application never runs as root. If `geteuid() == 0`, it refuses to start with an explanation. | planned |
| FM-APP-07 | A crash never leaves a half-applied file operation. Operations are journalled before the first byte is touched. | planned |

## 2. Window layout

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ [←][→][↑]  Home / Finance / 2024        [search]   [☰] [list|grid]      │ header
├──────────────┬───────────────────────────────────────────┬───────────────┤
│ Places       │ Name              Size    Modified  Kind │ Preview       │
│  Home        │ ▸ 2023/                           Folder   │ [thumbnail]   │
│  Documents   │ ▸ 2024/                           Folder   │               │
│  Downloads   │   Invoice_Acme.pdf  2.1 MB  12 Mar  PDF   │ name.pdf      │
│              │   budget.xlsx        48 KB   9 Jan  Sheet │ PDF · 2.1 MB  │
│ Learned      │                                           │               │
│  Documents   │                                           │ Why?          │
│  Images      │                                           │               │
│              │                                           │ [Open] [⋯]    │
├──────────────┴───────────────────────────────────────────┴───────────────┤
│ 14 items · 2 selected · 1.2 GB                  Indexed 4,812 · watching  │ status
└──────────────────────────────────────────────────────────────────────────┘
```

- Default window 1180×760; minimum 900×600.
- Sidebar 240–280 px, resizable; inspector 300–360 px, collapsible.
- Below 1000 px the inspector becomes an overlay sheet. Below 760 px the
  sidebar becomes a slide-over.
- The pane layout is per window, not per tab.
- A tab bar appears above the header only when more than one tab is open.

## 3. Navigation

| ID | Requirement | Status |
| --- | --- | --- |
| FM-NAV-01 | Each tab has an independent current directory, history (100 entries) and view mode. | planned |
| FM-NAV-02 | Back, forward and up are always visible, disabled when unavailable. `Alt+Left`, `Alt+Right`, `Alt+Up`. | planned |
| FM-NAV-03 | The location bar shows a breadcrumb by default and becomes an editable path on `Ctrl+L` or click. | planned |
| FM-NAV-04 | Typed paths expand `~`, resolve `.`/`..`, accept `file://` URIs, and show an inline error without navigating on invalid input. | planned |
| FM-NAV-05 | `Enter` in the location bar navigates; `Escape` restores the breadcrumb and current path. | planned |
| FM-NAV-06 | Double-click or `Enter` on a folder navigates into it; activating the current folder is a no-op. | planned |
| FM-NAV-07 | Dropping a folder onto the list navigates to it (open-in-place) rather than copying, unless a modifier is held. | planned |
| FM-NAV-08 | Places: Home, Documents, Downloads, Pictures, Music, Videos, Desktop, and any configured roots. Missing XDG directories are hidden. | planned |
| FM-NAV-09 | Mounted volumes appear under Places with free-space labels and an eject action; unmounting asks before closing tabs inside it. | planned |
| FM-NAV-10 | The window title is the current folder name followed by “Folderol”; the full path is in the location bar tooltip. | planned |
| FM-NAV-11 | `Ctrl+T` opens a new tab at the current folder; `Ctrl+W` closes the tab, not the window. Middle-click also closes. | planned |
| FM-NAV-12 | Opening a path from outside focuses an existing tab already showing it, or opens a new tab if `--new-tab`. | planned |
| FM-NAV-13 | The Trash place opens the XDG trash through GIO, including items trashed by other applications. | planned |
| FM-NAV-14 | Navigation never blocks the UI thread. Directory enumeration happens on a worker with a generation token; stale results are discarded. | planned |

## 4. Directory listing

| ID | Requirement | Status |
| --- | --- | --- |
| FM-LIST-01 | The listing comes from the filesystem, not the catalog. The catalog enriches rows; it never decides what exists. | planned |
| FM-LIST-02 | Folders sort before files by default. A setting can disable this. | planned |
| FM-LIST-03 | Names sort with natural, case-insensitive, locale-aware collation (`Report10` before `Report2`). | planned |
| FM-LIST-04 | Sort by name, size, modified, created (when available), kind, or category; ascending/descending; per-tab. | planned |
| FM-LIST-05 | Hidden entries are excluded by default. `Ctrl+H` toggles; the choice persists per window. | planned |
| FM-LIST-06 | A directory with 100k entries must remain scrollable and searchable while loading; rows stream in batches of 256. | planned |
| FM-LIST-07 | File names with invalid UTF-8 are displayed lossily but operated on with their original bytes. | planned |
| FM-LIST-08 | Broken symlinks are shown with a warning badge and can be deleted or repaired, never silently opened. | planned |
| FM-LIST-09 | Sockets, FIFOs and device nodes are shown but opening warns; they are never previewed by reading. | planned |
| FM-LIST-10 | The current directory is watched when possible; changes apply without reloading the view. A manual refresh exists for filesystems without notifications. | planned |
| FM-LIST-11 | Free space and item counts are shown in the status bar, not as a modal. | planned |
| FM-LIST-12 | Unreadable entries remain visible with their error; one unreadable file never fails the listing. | planned |

## 5. Views

### List (default)

Columns: Name, Size, Modified, Kind. Optional: Category, Confidence, Path.

- Name column flexes; others fixed by content with a 24 px minimum padding.
- Row height 44 px comfortable, 32 px compact.
- Selection uses accent at 12% fill plus a 2 px leading bar.
- Hover reveals one `⋯` button; all actions are also in the context menu and
  command palette.
- Category chip appears only at ≥70% confidence. Below that, no chip.
- Columns are resizable, reorderable and persist per window.

### Grid

- 168 px cards, 4:3 thumbnail, two-line name, category on hover.
- Default for folders containing ≥60% images/video, once chosen by the user;
  otherwise list remains the default.
- Grid selection is a contiguous block only with Shift; Ctrl-click is additive.
- Keyboard moves by row width, not linear index.

### Density and thumbnails

- Comfortable/compact is a global setting; list remembers per-tab.
- Thumbnails are generated off-thread through glycin, cached by
  `(path, mtime, size, 256px)`. A failed thumbnail shows the MIME icon.
- List uses MIME icons, not thumbnails, to keep scrolling cheap.

## 6. Selection

| ID | Requirement | Status |
| --- | --- | --- |
| FM-SEL-01 | Click selects; `Ctrl+Click` toggles; `Shift+Click` selects a range from the anchor. | planned |
| FM-SEL-02 | `Ctrl+A` selects all, `Ctrl+Shift+A` clears, `Ctrl+I` inverts. | planned |
| FM-SEL-03 | Arrow keys move the cursor and select; `Shift+Arrow` extends; `Ctrl+Space` toggles the cursor row. | planned |
| FM-SEL-04 | Typing a printable character starts type-ahead find; `Escape` cancels. | planned |
| FM-SEL-05 | The status bar reports item count and total size of the selection, accounting for symlinks without following them. | planned |
| FM-SEL-06 | Selection survives a refresh; rows that disappear are dropped silently. | planned |
| FM-SEL-07 | Multi-selection of folders is allowed for move/copy/trash, and disallowed for Open and Properties (disabled with a tooltip). | planned |

## 7. Opening files

| ID | Requirement | Status |
| --- | --- | --- |
| FM-OPEN-01 | `Enter` or double-click opens a file with its default handler via GIO/portal. | planned |
| FM-OPEN-02 | `Ctrl+Enter` opens the containing folder and selects the file (Reveal). | planned |
| FM-OPEN-03 | “Open with…” lists registered handlers and remembers the choice per MIME type only when the user checks “Always”. | planned |
| FM-OPEN-04 | Executable files prompt once per session before running, and never run when the file is group/world-writable without a warning. | planned |
| FM-OPEN-05 | Opening more than 10 files asks for confirmation; opening more than 50 is disabled with an explanation. | planned |
| FM-OPEN-06 | Failure to launch shows the exact GIO error and offers “Open with…”. | planned |
| FM-OPEN-07 | “Open Terminal Here” uses `xdg-terminal-exec` when present, then `$TERMINAL`, then the first available of `ghostty`, `alacritty`, `foot`, `konsole`, `gnome-terminal`. | planned |
| FM-OPEN-08 | “Copy path” copies the absolute path; “Copy as URI” copies `file://`. | planned |

## 8. Clipboard and drag and drop

| ID | Requirement | Status |
| --- | --- | --- |
| FM-DND-01 | `Ctrl+C`/`Ctrl+X`/`Ctrl+V` operate on files through GIO, preserving the `x-special/gnome-copied-files` clipboard format for interoperability. | planned |
| FM-DND-02 | Internal drag moves by default; `Ctrl` copies; `Shift` moves; `Alt` creates a link with confirmation. | planned |
| FM-DND-03 | External drops accept `text/uri-list`; dropping into the window copies. | planned |
| FM-DND-04 | Dragging onto a folder row springs it open after 600 ms, then continues into it. | planned |
| FM-DND-05 | Auto-scroll begins within 24 px of the list edge and accelerates with proximity. | planned |
| FM-DND-06 | Dropping onto a file targets its parent folder. | planned |
| FM-DND-07 | Dropping onto empty space in the list targets the current folder. | planned |
| FM-DND-08 | A drag from a privileged location into a normal one is allowed; the reverse fails with a permission message, never a partial copy. | planned |
| FM-DND-09 | Dragging out of Folderol exports the same URI list understood by Nautilus and Dolphin. | planned |

## 9. File operations

All operations go through GIO and the operation engine (§10). No UI code calls
`std::fs::remove_*` or `rename` on user files.

| ID | Operation | Behaviour | Status |
| --- | --- | --- | --- |
| FM-OPS-01 | New folder | Inline name in the list, created with `0755 & ~umask`. Invalid names rejected locally. | planned |
| FM-OPS-02 | New empty file | Same flow; extension optional; refuses reserved names. | planned |
| FM-OPS-03 | Rename | Inline edit with the stem selected, extension preserved. `F2`. | planned |
| FM-OPS-04 | Batch rename | Pattern with `{name}`, `{ext}`, `{n}`, `{date}`, plus find-and-replace. Live preview of every result and collisions. | planned |
| FM-OPS-05 | Duplicate | Creates “name (copy).ext”, then “(copy 2)”. Never overwrites. | planned |
| FM-OPS-06 | Copy / move | Recursive, preserves mtimes and permissions; follows the collision policy. | planned |
| FM-OPS-07 | Trash | XDG trash through GIO, with restore metadata. `Delete`. | planned |
| FM-OPS-08 | Delete permanently | Only via `Shift+Delete`, with a typed confirmation for directories and a clear “cannot be undone” label. Never the default. | planned |
| FM-OPS-09 | Restore from trash | Restores to the original path if free; otherwise asks for a destination. | planned |
| FM-OPS-10 | Empty trash | Confirmation with item count and total size; irreversible. | planned |
| FM-OPS-11 | Properties | Size on disk, item count, timestamps, MIME, owner, group, permissions, path, inode. Permissions editable for owned files. | planned |
| FM-OPS-12 | Symlink | Create relative or absolute link with a name prompt; warn when the target does not exist. | planned |
| FM-OPS-13 | Rename collision | Ask: Skip, Replace, Keep both, Cancel. “Apply to all” scoped to the current operation. | planned |
| FM-OPS-14 | Cross-filesystem move | Copy to a temporary name, verify size and checksum for files over 64 MB, rename, then delete the source. On any failure the source remains. | planned |
| FM-OPS-15 | Operation cancellation | Cancelling leaves completed items completed, removes partial temporary files, and reports exactly what happened. | planned |
| FM-OPS-16 | Read-only destination | Fails before starting with a clear message; never partially copies. | planned |
| FM-OPS-17 | Name validation | Rejects `/`, NUL, empty, `.` and `..`; warns on leading `-` and trailing spaces; allows Unicode. | planned |
| FM-OPS-18 | Compress / extract | v1.1. ZIP and tar first; no archive mounting. | deferred |
| FM-OPS-19 | Duplicate finder and disk usage | v1.1, read-only analysis surfaces. | deferred |

## 10. Operation engine and undo

| ID | Requirement | Status |
| --- | --- | --- |
| FM-ENG-01 | Operations run in a queue with at most two active filesystem workers; UI stays responsive. | planned |
| FM-ENG-02 | Every operation reports progress (items and bytes), current path, elapsed time and estimated remaining. | planned |
| FM-ENG-03 | The operation centre is a popover from the status bar; it survives navigation and tab changes. | planned |
| FM-ENG-04 | Pause is supported for copy/move when the backend can pause; otherwise the control is absent, not disabled fake. | planned |
| FM-ENG-05 | Undo covers rename, move, copy, new folder/file, and trash. `Ctrl+Z` restores the most recent operation for the session. | planned |
| FM-ENG-06 | The undo journal is written to SQLite before the operation begins and removed after success, so a crash can be recovered on next launch. | planned |
| FM-ENG-07 | Permanent deletion is never undoable and is labelled as such. | planned |
| FM-ENG-08 | Failed items are collected into an error report with source, destination and OS error; the rest of the batch continues. | planned |
| FM-ENG-09 | Operations on the same path are serialised; unrelated operations may proceed in parallel. | planned |
| FM-ENG-10 | A destination inside the source (e.g. copying a folder into itself) is detected and refused. | planned |

## 11. Trash

- Uses the freedesktop.org trash specification via GIO: `~/.local/share/Trash`
  and per-volume `.Trash-$uid`.
- Restore preserves the original path and deletion time.
- Items on removable media that is absent appear as “unavailable” with the
  mount point, not as an error dialog.
- Emptying trash reports how many items and bytes were freed.
- Folderol never permanently deletes a file as a side effect of another
  operation. “Replace” removes the destination only after the new file is fully
  written and verified.

## 12. Search

### Field behaviour

| ID | Requirement | Status |
| --- | --- | --- |
| FM-SRCH-01 | The search field searches the whole index by default; a scope chip switches to “This folder” or a selected root. | planned |
| FM-SRCH-02 | Query syntax: bare words, `"quoted phrase"`, `type:pdf`, `in:Finance`, `before:2024-01-01`, `after:`, `size:>10MB`, `category:documents`, `is:hidden`. | planned |
| FM-SRCH-03 | Invalid scopes produce an inline hint; they never silently broaden the query. | planned |
| FM-SRCH-04 | Results appear within 30 ms p95 at 100k files, streamed if a slower backend is used. | planned |
| FM-SRCH-05 | Results are grouped: **Best matches**, **In this folder**, **Similar files**. Empty groups are omitted. | planned |
| FM-SRCH-06 | Each result shows a one-line reason: `matched name and folder`, `same meaning, different words`, `recently used`. | planned |
| FM-SRCH-07 | Selecting a result previews it; `Enter` opens; `Ctrl+Enter` reveals; `Ctrl+C` copies the path. | planned |
| FM-SRCH-08 | Search history is local, capped at 20, and clearable. No query leaves the machine. | planned |
| FM-SRCH-09 | Saved searches appear in the sidebar and restore their query and scope. | planned |
| FM-SRCH-10 | A search that returns nothing offers the active scopes, a “search everywhere” action, and the indexing state. | planned |
| FM-SRCH-11 | Ranking weights can be inspected and adjusted in a small popover; defaults reset. | planned |
| FM-SRCH-12 | Search never blocks on the embedder warming up; lexical results appear first and semantic results merge in. | planned |

### Ranking

```text
score = 1.00 · cosine(query, file)
      + 0.90 · rank_score(FTS5)
      + 0.15 · category_confidence
      + 0.05 · recency
```

A hit survives only with lexical support or a cosine above the embedder's
`semantic_floor`. Grouping is presentational; the underlying list is one ranked
set. See `ARCHITECTURE.md` § Retrieval and `ML.md` § Ranking.

## 13. Semantic surfaces

| ID | Requirement | Status |
| --- | --- | --- |
| FM-SEM-01 | Categories are shown as chips only at ≥70% confidence. | planned |
| FM-SEM-02 | The inspector's **Why this category?** lists name tokens, path tokens, extension and prior, plus human labels that changed the answer. | planned |
| FM-SEM-03 | Correcting a category is two clicks: chip → menu, or drag a row onto a sidebar category. | planned |
| FM-SEM-04 | After a correction, a toast offers “Apply to N similar files” with Review and Dismiss. Nothing is applied without confirmation. | planned |
| FM-SEM-05 | Low-confidence files are never badged as errors; they simply have no chip, and the filter `confidence:<70%` can find them. | planned |
| FM-SEM-06 | Smart folders are proposed in the sidebar under “Suggested”, never created silently. | planned (M4) |
| FM-SEM-07 | A settings action resets the personal model (`model.json`) without touching files or labels; a second action deletes labels. | planned |
| FM-SEM-08 | The model never initiates a move, rename or delete. | shipped as a principle; enforced from M3 |

## 14. Sidebar

- **Places** — XDG folders, configured roots, pinned folders, volumes, Trash.
  Drag to reorder; right-click to remove a pin.
- **Learned** — categories with counts, ordered by use. Categories with fewer
  than three files collapse under “More”. Clicking filters the index by
  category, not the current directory.
- **Saved** — saved searches and, from M4, pinned smart folders.
- **Tags** — M4. Until then the section is absent, not an empty stub.
- Footer: index size, files indexed, watch status, and a link to Settings →
  Indexing.

## 15. Inspector

- Preview: images (glycin), video poster + play, PDF first page, text/code with
  syntax highlighting up to 1 MB, audio waveform, font specimen, fallback MIME
  icon + metadata.
- Metadata: size and size on disk, modified, created, accessed, MIME, owner,
  group, permissions, inode, target for symlinks.
- Multi-selection: total count and size, common parent, no preview.
- Actions: Open, Open with, Reveal, Rename, Move to Trash, Properties.
- **Why?** panel is collapsed by default; expanding it is instant and never
  spins.
- Preview generation is cancellable and bounded; scrolling rapidly never queues
  500 previews.

## 16. Command palette

`Ctrl+K`. One input, three sections:

1. **Actions** — every menu action with its shortcut.
2. **Files** — top five hybrid results.
3. **Places** — paths and sidebar locations.

Fuzzy matching highlights matched characters. `Enter` runs the first result.
`Tab` completes a path. `Escape` closes. The palette never opens a nested mode.

## 17. Status bar and operation centre

- Left: item count, selection count and size.
- Right: index state (`Indexed 4,812 · watching`), free space, operation
  spinner.
- Clicking the index state opens the indexing panel: roots, last scan, errors,
  model version, “Rescan”, “Rebuild embeddings”.
- Operations appear in a popover with per-item progress, cancel, and a link to
  the error report.

## 18. Settings

| Page | Contents |
| --- | --- |
| Appearance | theme (system/dark/light), density, grid card size, show category chips, reduced motion |
| Behaviour | folders first, single-click open, default view, restore session, confirm trash, terminal command |
| Indexing | roots, exclusions, hidden files, follow symlinks, cross filesystems, default excludes, content extraction (off), rescan, rebuild |
| Learning | categories with label counts, confidence threshold, reset model, delete labels |
| Privacy | index location and size, delete index, export diagnostics (local file), no network statement |
| Advanced | ranking weights, log level, database path (read-only display), copy diagnostics |

Settings are stored in `$XDG_CONFIG_HOME/folderol/settings.toml`, versioned with
a `schema_version` key. Unknown keys are preserved. Invalid values fall back to
defaults with a visible warning in the settings UI.

## 19. Errors and edge cases

| Situation | Behaviour |
| --- | --- |
| Permission denied opening a folder | Show the path, the OS error and the owning user/group if available; offer “Try as administrator” **never**. |
| File vanished during operation | Report as skipped, continue; do not treat the batch as failed. |
| Read-only filesystem | Disable mutating actions with a tooltip before the user tries. |
| Destination full | Stop before writing the next item, report bytes written, leave partial temp file removed. |
| Removable volume removed mid-copy | Mark the operation interrupted, keep the source, explain which volume disappeared. |
| Name too long | Validate against `NAME_MAX` and `PATH_MAX` before starting; suggest a shortened stem. |
| Case-only rename on a case-insensitive filesystem | Perform a two-step rename through a temporary name. |
| Symlink loop | Refuse to follow; show the chain length. |
| FUSE/network latency | Show a busy row, never freeze the window; timeout reads after 30 s with a retry. |
| Index unavailable/corrupt | Browsing still works; search shows “index unavailable” with rebuild and delete actions. |
| Model file missing | Search falls back to lexical; categories fall back to priors. Never a startup error. |
| Clock skew / future mtime | Display as-is; recency ranking clamps to now. |
| Trash on a volume without `.Trash` | Ask whether to delete permanently or move to home trash. |

## 20. Platform integration

- XDG user directories resolved through `xdg-user-dir`, not hardcoded.
- MIME through GIO, never a private extension table for opening.
- `io.github.samiulh25.Folderol.desktop` registers `inode/directory` and
  `x-scheme-handler/folderol`, with `%U`.
- Desktop actions: New Window, Search.
- Single instance via `GApplication`; command-line handoff carries paths.
- Portals are preferred for file chooser and trash when sandboxed.
- Wayland first; X11 supported through GDK. No global shortcuts, no blur
  dependency.
- The index lives in `$XDG_DATA_HOME/folderol/`; settings in
  `$XDG_CONFIG_HOME/folderol/`; caches in `$XDG_CACHE_HOME/folderol/`.

## 21. Accessibility

- Every pointer action has a keyboard equivalent and appears in the palette.
- Focus order: sidebar → location bar → search → list → inspector.
- Rows expose name, kind, size, modified and category confidence to AT-SPI.
- Operation progress is announced through a live region at start, 50%, and end;
  not on every percent.
- Contrast ≥ 4.5:1 for text and ≥ 3:1 for focus.
- Category is never colour-only.
- Reduced motion collapses animations to a 100 ms opacity change.
- Hit targets ≥ 32 px; text scales with system font settings up to 200% without
  clipping.

## 22. Performance budgets

| Surface | Budget |
| --- | --- |
| Window first paint | ≤ 400 ms warm |
| Directory first 256 rows | ≤ 100 ms on a local SSD |
| Scroll | 60 fps with 10k rows |
| Search keystroke to results | ≤ 50 ms input-to-paint at 100k files |
| Thumbnail first display | ≤ 150 ms for a 4 MB JPEG |
| Memory, 10k-row folder | ≤ 200 MB resident |
| Memory, 100k indexed | ≤ 300 MB resident |
| Operation UI updates | ≤ 10 Hz per operation, coalesced |

## 23. v1.0 acceptance checklist

- [ ] Browse `/`, `$HOME`, a removable drive and a 100k-entry directory.
- [ ] Create, rename, batch rename, copy, move, trash, restore and undo.
- [ ] Kill the app during a copy; on restart the journal explains and recovers.
- [ ] Search by word, phrase, scope and description; every result explains why.
- [ ] Correct 20 files; suggestions improve without a single automatic move.
- [ ] Run with no index and with a corrupt index; browsing still works.
- [ ] Keyboard-only walkthrough of every primary task.
- [ ] Screen reader walkthrough of list, search and operation progress.
- [ ] 24-hour soak with a watcher on `$HOME`; no leaked file descriptors, no
      unbounded memory.
- [ ] Package installs on a clean Arch VM and a clean Flatpak runtime.
- [ ] Delete the index and verify no user file was touched.

## 24. Deferred with intent

- Tabs groups, split view, miller columns.
- Archive browsing, network locations, MTP devices.
- File content search (opt-in extraction is M2; the UI ships before the index
  claims it).
- Duplicate finder, disk-usage treemap, checksum tool.
- Plugin API.
- Windows/macOS.

Each deferred item needs a new ADR before it returns to the roadmap.
