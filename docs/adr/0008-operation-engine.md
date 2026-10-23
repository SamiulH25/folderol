# ADR 0008 — A single file-operation engine with an undo journal

- Status: proposed
- Date: 2026-10-23

## Context

Folderol will copy, move, rename and trash files. Each of those can destroy data
if it is implemented ad hoc in a widget callback. The browser also needs
progress, cancellation, collision handling and undo, all of which are easier to
get right once than five times.

## Decision

All mutations go through one operation engine:

- GIO for filesystem operations, never raw `std::fs::remove_*` on user files.
- XDG trash for deletion; permanent delete only behind `Shift+Delete` and a
  typed confirmation.
- A queue with at most two active workers, operations serialised per path.
- A journal row written to SQLite before the first mutation, removed after
  success. On launch, unfinished rows become a recovery prompt.
- Copy to a temporary name, verify, then rename into place; cross-filesystem
  moves copy-verify-delete and never delete the source on failure.
- Undo is available for rename, move, copy, create and trash for the session and
  across a crash via the journal. Permanent delete is never undoable.
- Errors are per item: the batch continues, and a review list reports source,
  destination and reason.

The UI observes the engine through a snapshot: items done, bytes done, current
path, errors. It never performs filesystem work itself.

## Alternatives considered

- **Direct calls in each widget.** Fastest to write, impossible to test
  uniformly, and every new surface reintroduces the same collision and error
  bugs. Rejected.
- **A separate privileged helper.** Could survive crashes and run operations
  outside the UI process. Adds IPC, authentication and a new attack surface;
  unnecessary for same-user file management. Rejected for 1.0.
- **Shelling out to `cp`/`mv`/`gio`.** Tempting for simplicity, but parsing
  progress and errors from localized output is brittle, and filenames with
  newlines break naive quoting. Rejected.
- **No undo, only confirmation dialogs.** Confirmation fatigue makes people
  click through; undo is the safer pattern. Rejected.
- **Persistent operation daemon.** Revisit only if operations must continue
  after the window closes.

## Consequences

- Every mutating feature has one test surface and one place to fix bugs.
- The journal adds a schema concern and a recovery UI, but makes crash safety
  demonstrable.
- GIO's asynchronous API and portal behaviour become part of the engine's
  contract; platform differences are handled once.
- Copying across devices is slower because of verification, which is the correct
  trade for user data.
- Undo has limits: it restores a path, not inode identity, and cannot restore a
  file that another process modified after the undo window. The UI does not
  pretend otherwise.
