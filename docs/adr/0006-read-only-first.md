# ADR 0006 — Read-only first, safe mutations later

- Status: accepted
- Date: 2026-10-23

## Context

A file browser is trusted with the user's data. A bug in navigation is an
annoyance; a bug in move or delete is a catastrophe. The intelligence layer is
also new and will occasionally be wrong.

## Decision

Milestone 0–2 are read-only: scan, browse, search, preview, learn. No file is
created, renamed, moved or deleted by Folderol.

The first mutating milestone (M3) must satisfy all of:

1. Mutations go through GIO (`gio::File`), never bespoke path manipulation.
2. Deletion means the XDG trash specification, never `unlink` on user files.
3. Every destructive action is visible, confirmed when ambiguous, and undoable
   for ten seconds via a toast.
4. Moves are same-filesystem `rename` first, with a copy-then-verify-then-delete
   fallback across devices.
5. The model never triggers a mutation. It may propose; the user acts.
6. A failed operation reports the exact path and the OS error, and leaves the
   source untouched.

## Consequences

- Early users cannot lose data through Folderol, which matters more than
  feature parity while the engine is young.
- The "apply to 11 similar files" flow learns first, mutates only after explicit
  confirmation, and is implemented after undo exists.
- Some reviewers will call it incomplete. That is the correct trade at this
  stage.
- Undo needs a small journal (operation id, source, destination, timestamp).
  It belongs in the catalog, not in memory, so a crash mid-toast does not lose
  the ability to restore. Trashed files are restored by the desktop's trash
  implementation, not by us.
- Network filesystems are treated as hostile: no inode assumptions, no atomic
  rename assumption, and no automatic cleanup.
