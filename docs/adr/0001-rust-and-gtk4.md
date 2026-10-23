# ADR 0001 — Rust and GTK4 + libadwaita

- Status: accepted
- Date: 2026-10-23

## Context

Folderol is a Linux-native file browser. It must render directories with 100k+
entries, support multi-select, drag and drop, context menus, thumbnails, IME and
screen readers, and look deliberate on both Wayland and X11. The intelligence
layer is CPU-bound Rust-friendly work (vector math, SQLite, later ONNX).

The realistic candidates were GTK4, Qt6/Kirigami, iced, Slint, and a web shell
(Tauri/Electron).

## Decision

Rust 2024 with `gtk4-rs` and `libadwaita`, styled with a custom CSS layer. GTK
owns the shell, input, accessibility and platform integration. No web view.

## Alternatives considered

- **Qt6 + Kirigami.** Equally capable and very themeable. Rejected because it
  splits the project across C++ and QML, and the ML ecosystem we need is
  Rust-first. PySide would add a runtime and an FFI boundary for the hot path.
- **iced.** Best-looking pure-Rust option and full control over rendering.
  Rejected because it would require rebuilding virtualized lists, multi-select,
  IME, accessibility and native drag and drop — the exact primitives a file
  browser cannot get wrong.
- **Slint.** Lightweight and pleasant, but a thinner ecosystem and less mature
  platform integration for a file manager.
- **Tauri/Electron.** Rejected: memory cost, input latency, inconsistent
  Wayland behaviour, and filesystem access mediated by a web layer is the wrong
  trust boundary for a tool that can delete files.

## Consequences

- Build requires GTK4 and libadwaita development headers. This is why
  `folderol-gui` is excluded from the default workspace build.
- libadwaita sets a version floor (currently 1.4 for `ToolbarView`; the shell
  targets 1.9). Older distros need Flatpak.
- The visual identity is ours, but the platform conventions (header bars,
  shortcuts, file chooser) come for free.
- GTK types are confined to `folderol-gui`; the engine stays UI-agnostic and
  testable headless.
- Wayland is the primary target. X11 works through the same GDK backend, but
  global shortcuts and blur are explicitly not dependencies.
