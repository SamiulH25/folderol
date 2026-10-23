# Security

## Reporting

Please do not open a public issue for a vulnerability. Use the repository host's
private security advisory feature; if that is unavailable, contact the
maintainer directly and include a minimal reproduction. You will get an
acknowledgement within a few days.

## Threat model

Folderol runs with the user's privileges and can read every path the user can
read. The design reduces blast radius rather than claiming sandboxing:

- **No network code in the engine.** There is no telemetry, remote logging, sync
  or model inference service. A model download, when added, is a separate,
  explicit action with a pinned SHA-256.
- **Metadata only by default.** Scanning does not copy file contents into the
  catalog. Content extraction is opt-in per root and stored separately.
- **Symlinks are not followed** while walking, which prevents cycles and
  traversal outside a selected root.
- **The catalog is sensitive.** It contains full paths and learned labels.
  It is created with `0600` under `$XDG_DATA_HOME/folderol/`. Deleting it is a
  supported operation; back it up like any private file.
- **No mutation before M3.** The current code creates and updates its own
  catalog only. File operations arrive with the safety rules in ADR 0006:
  GIO, XDG trash, visible confirmation, undo.

## Out of scope

- A malicious local user with the same account. They can read the files anyway.
- A compromised desktop session, keylogger or screen reader.
- Vulnerabilities in GTK, SQLite or the kernel. Report those upstream, but tell
  us if Folderol's usage makes them worse.

## Hardening checklist for contributors

- Never build shell commands from file paths.
- Never construct FTS5 queries from raw input; tokenize first.
- Treat SQLite as untrusted input: validate dimensions and lengths before
  decoding vectors.
- Do not log full paths at info level in release builds.
- New dependencies must be justified in the pull request and must not add
  network access to the engine.
