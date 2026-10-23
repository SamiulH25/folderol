# State and copy specification

**Status:** target for v1.0, with shipped M0 states marked.

Every state is a first-class screen, not a fallback. A file manager spends a
surprising amount of time loading, failing and being empty; those moments decide
whether it feels trustworthy.

## 1. Principles

1. **No dead ends.** Every empty or error state offers the next useful action.
2. **No fake progress.** A spinner without a determinate count is used only when
   the work is genuinely unknown; scanning and copying always show items/bytes.
3. **No blame.** “Permission denied” explains the owner, not “you did it wrong”.
4. **No exclamation marks.** Errors are statements, not alarms.
5. **Nothing disappears silently.** Failed items remain visible with their error.
6. **Reduced motion** turns every transition into a 100 ms opacity change.
7. **Screen readers hear the outcome**, not every percentage.

## 2. Application lifecycle

| State | Trigger | UI | Recovery |
| --- | --- | --- | --- |
| First launch | no settings, no index | Welcome window: three bullets, “Choose folders to index”, “Browse without indexing” | either action closes the sheet |
| Warm launch | index present | Last location or Home; rows stream in | — |
| Index missing | DB deleted | Browse works; search field shows a one-line inline notice | “Build index” |
| Index corrupt | open/migrate fails | Browse works; banner: “Search is unavailable because the index could not be opened.” | “Rebuild index”, “Open index folder” |
| Model missing | ONNX selected but absent | Search works lexically; badge “Local model not installed” | “Install model” in Settings |
| Model warming | first semantic query | Inline progress row under the search field, ≤ 1 s | results merge when ready |
| Indexing in background | scan/watch active | Status bar count + pulse; no modal | click for details |
| Single instance handoff | second launch with a path | Focus existing window, open tab | — |

## 3. Directory and search states

| State | UI | Copy | Actions |
| --- | --- | --- | --- |
| Loading directory | Skeleton rows, path bar active | — | Cancel by navigating elsewhere |
| Empty folder | Centered icon + heading | “This folder is empty.” | “New Folder”, “Paste” (enabled only with clipboard content) |
| Loading 100k rows | First 256 rendered, subtle determinate bar | “Loading 12,480 of 100,000…” | Scroll immediately; cancel not needed |
| Permission denied | Icon + path + owner/group | “You don’t have permission to open **/var/lib/private**.” | “Properties”, “Open parent” |
| Folder vanished | Banner over previous contents | “**/mnt/usb/photos** is no longer available.” | “Go to parent”, “Remove from recent” |
| No search results | Icon + query echo | “No files match **lease flat**.” | “Search everywhere”, “Clear filters”, “Indexing settings” |
| Search still warming | Lexical results + inline row | “Showing word matches while the local model warms up.” | — |
| Search index empty | Icon | “Nothing is indexed yet.” | “Choose folders”, “Browse files” |
| Category empty | Icon | “No files in **Documents** yet.” | “Show all files”, “Rescan” |
| Trash empty | Icon | “Trash is empty.” | — |
| Selection empty | Inspector placeholder | “Select a file to see details.” | — |
| Multi-selection | Summary | “14 items · 1.2 GB selected” | “Trash”, “Copy”, “Properties” disabled where meaningless |
| Saved search malformed | Sidebar row warning | “This saved search uses a filter that no longer exists.” | “Edit”, “Delete” |

## 4. Index and scanning states

| State | Where | UI | Copy |
| --- | --- | --- | --- |
| Fresh index | status bar | neutral dot | “Not indexed” → “Choose folders” |
| Scanning | status bar + panel | determinate bar | “Indexing 12,480 / 100,000 · 4,120/s” |
| Partial scan | panel banner | amber dot | “Last scan stopped early. 12,480 files indexed.” → “Continue” |
| Scan cancelled | toast | neutral | “Scan stopped. 12,480 files indexed.” → “Continue” |
| Watching | status bar | small pulse | “Watching 2 folders” |
| Watch unsupported | status bar | hollow dot | “This folder can’t be watched. Refresh manually.” |
| Watch limit reached | panel banner | amber | “The system’s file-watch limit was reached. Indexing still works; live updates paused.” → “How to fix” |
| Model changed | panel banner | amber | “Embeddings changed. A full rebuild is required.” → “Rebuild” |
| Rebuilding | panel | determinate | “Rebuilding embeddings 8,200 / 100,000” |
| Index full disk | panel banner | red | “The disk holding the index is full. Indexing paused.” → “Open settings” |
| Index too large (warning) | panel | neutral | “The index is 2.4 GB. You can exclude folders or delete it.” |

## 5. Operation and undo states

| State | UI | Copy |
| --- | --- | --- |
| Queued | operation row, “Queued” | “Waiting for 2 other operations” |
| Copying | determinate bar, current file | “Copying 412 of 1,200 · 1.8 GB of 4.2 GB” |
| Paused | paused badge | “Paused” |
| Cancelling | bar settles, no new items | “Stopping after the current file…” |
| Cancelled | summary | “Cancelled. 412 files copied, 3 skipped.” |
| Partial failure | amber row + “Review” | “Finished with 7 problems. 1,193 of 1,200 items completed.” |
| Completed | toast, auto-dismiss 6 s | “Copied 1,200 items to **Archive**.” → “Undo” |
| Trashed | toast, 10 s | “Moved 3 items to Trash.” → “Undo”, “Show Trash” |
| Renamed | toast, 10 s | “Renamed to **Invoice_Acme_2024.pdf**.” → “Undo” |
| Undo failed | amber toast | “Could not undo. The destination already exists.” → “Review” |
| Operation crashed | on next launch | “A move was interrupted. 214 of 400 items were completed.” → “Review”, “Dismiss” |

## 6. Confirmation dialogs

Only for irreversible or ambiguous actions. Everything else is undoable and
uses a toast.

| Action | Title | Body | Buttons |
| --- | --- | --- | --- |
| Move to Trash | none by default; confirmation when the setting is on | toast with Undo, or the confirm dialog | Undo / Cancel / Trash |
| Delete permanently | “Delete permanently?” | “**name** will be deleted immediately. This cannot be undone.” | Cancel / Delete |
| Delete a folder permanently | “Delete **name** permanently?” | “This folder and its 412 items will be deleted. This cannot be undone.” Type the folder name to confirm. | Cancel / Delete |
| Empty Trash | “Empty Trash?” | “412 items (1.8 GB) will be deleted permanently.” | Cancel / Empty Trash |
| Replace on collision | “An item named **name** already exists.” | “Replace it with the new copy?” | Skip / Keep both / Replace |
| Apply to similar | “Apply **Documents** to 11 similar files?” | List of files with confidence. | Cancel / Review / Apply |
| Close with operations | “An operation is still running.” | “Closing now will cancel 412 remaining items.” | Keep open / Cancel operation and close |
| Download model | “Install the local model?” | “23 MB · Apache-2.0 · downloaded once, verified by SHA-256. No file data is sent.” | Cancel / Install |
| Reset personal model | “Reset learned categories?” | “Labels and corrections are kept. Only the trained model is deleted.” | Cancel / Reset |
| Delete labels | “Delete all corrections?” | “This removes 214 labels. Files are not touched.” | Cancel / Delete labels |
| Delete index | “Delete the index?” | “Search and categories stop working. Your files are not touched.” | Cancel / Delete index |

## 7. Toasts

- Max width 420 px, bottom center, 10 s for undoable actions, 6 s otherwise.
- One toast at a time; a new one replaces the old and the old action remains in
  the operation centre.
- No toast for background scan progress.
- Undo is always the first action, never hidden behind a menu.

Copy patterns:

```text
Moved 3 items to Trash.                    [Undo] [Show Trash]
Copied 1,200 items to Archive.             [Undo]
Renamed to Invoice_Acme_2024.pdf.          [Undo]
Could not move 2 items.                    [Review]
Index updated: 4,812 files.                [Details]
```

## 8. Error copy rules

- Name the object in bold, the operation, and the OS reason in plain language.
- Never include a raw errno without a human sentence; include the errno in the
  details expander.
- Offer exactly one primary recovery and at most two secondary actions.
- Do not blame the user. “The file is read-only” not “You tried to edit a
  read-only file”.

Examples:

```text
Couldn’t open **report.pdf**
The file is no longer there. It may have been moved or deleted.
[Go to parent] [Search for it]

Couldn’t copy **video.mkv**
The destination disk is full. 1.4 GB is needed.
[Choose another folder] [Cancel]

Couldn’t read **/mnt/usb**
The device was removed. Reconnect it and try again.
[Retry] [Go to Home]
```

## 9. Accessibility announcements

| Event | Announced |
| --- | --- |
| Directory loaded | “Finance, 42 items, 3 selected” |
| Search finished | “18 results for invoice acme, best match Invoice_Acme_2024.pdf” |
| Operation start | “Copying 1,200 items to Archive” |
| Operation 50% | “Copy 50 percent complete” |
| Operation end | “Copied 1,200 items. 7 problems.” |
| Error | the visible error text, once |
| Toast undo window | “Moved 3 items to Trash. Undo available for 10 seconds.” |

Never announce every progress tick. Use `polite` for progress, `assertive` for
failures that stop work.

## 10. Empty-state illustrations

- Monochrome symbolic icon, 64 px, 40% opacity.
- One sentence, one optional secondary sentence, one primary action.
- No mascots, no jokes, no “Oops!”.
- The illustration is decorative and hidden from assistive technology.

## 11. Reduced motion and high contrast

- Reduced motion: no slides, no scale, no spring; fade 100 ms or instant.
- High contrast: remove translucency, raise border contrast to 30%, keep accent
  for focus only.
- Neither mode changes layout or information density.

## 12. Copy tone

- Sentence case for headings and buttons.
- “Couldn’t” rather than “Failed to” in user-facing prose; logs use precise
  technical terms.
- File and folder names in **bold**.
- Numbers formatted with the locale’s grouping, but sizes use binary units
  (`KB`, `MB`, `GB`) with one decimal below 10 and none above.
- No time estimates until at least three items have completed and the rate is
  stable; then round to a human unit (“about 40 seconds”).
