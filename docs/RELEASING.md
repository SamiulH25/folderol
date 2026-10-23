# Releasing Folderol

**Status:** process definition. The first public release will be **0.2.0**
(M1). Until then, tags are development snapshots and no compatibility is
promised.

## 1. Versioning

- Semantic versioning for the workspace: `MAJOR.MINOR.PATCH`.
- Before 1.0: `MINOR` may break CLI flags, config keys and the database schema;
  `PATCH` is fixes only.
- After 1.0: the CLI, settings schema and catalog schema follow semver. Model
  ids are independent and always versioned.
- One version for every crate in the workspace, bumped in
  `[workspace.package] version`.
- Git tags are `vX.Y.Z`, annotated and signed.
- `main` is always releasable. Release work happens on `release/X.Y` only after
  a freeze.

## 2. Identity

| Item | Value |
| --- | --- |
| Application ID | `io.github.samiulh25.Folderol` |
| CLI binary | `/usr/bin/folderol` |
| GUI binary | `/usr/bin/folderol-gui` |
| Desktop file | `/usr/share/applications/io.github.samiulh25.Folderol.desktop` |
| AppStream | `/usr/share/metainfo/io.github.samiulh25.Folderol.metainfo.xml` |
| Icon | `/usr/share/icons/hicolor/scalable/apps/io.github.samiulh25.Folderol.svg` |
| Data | `$XDG_DATA_HOME/folderol/` |
| Config | `$XDG_CONFIG_HOME/folderol/` |
| Cache | `$XDG_CACHE_HOME/folderol/` |
| License | GPL-3.0-or-later |

The application ID is part of the desktop identity. Changing it orphans
settings, caches and window rules, so it does not change after 0.2.

## 3. Channels

| Channel | Source | Audience | Cadence |
| --- | --- | --- | --- |
| `main` | git | contributors | continuous |
| nightly | CI artifact from `main` | testers | on demand |
| beta | tagged `vX.Y.Z-beta.N` | early adopters | before minor releases |
| stable | AUR `folderol`, Flatpak | everyone | minor releases |
| git package | AUR `folderol-git` | bleeding edge | follows `main` |

No channel collects telemetry. Beta users are asked for feedback in the issue
tracker, not instrumented.

## 4. Release checklist

### Before the freeze

- [ ] All milestone acceptance criteria in `ROADMAP.md` are met.
- [ ] `docs/` reflects shipped behaviour; no “planned” claim is stale.
- [ ] `cargo test --workspace`, `cargo clippy -D warnings`, `cargo fmt --check`.
- [ ] Golden retrieval metrics recorded and within gates.
- [ ] Benchmark recorded: 100k index, query p50/p95, scan time, RSS, DB size.
- [ ] Manual GUI matrix completed on Wayland and X11.
- [ ] Accessibility checklist completed; at least one Orca pass.
- [ ] Crash/restart test during copy and move.
- [ ] Migration test from the previous released schema.
- [ ] `CHANGELOG.md` updated.
- [ ] Screenshots captured at 2× for AppStream and release notes.

### Version bump

```bash
# workspace version
$EDITOR Cargo.toml
# version + release date in AppStream
$EDITOR data/io.github.samiulh25.Folderol.metainfo.xml
# changelog
$EDITOR CHANGELOG.md

cargo test --workspace
git commit -am "release: v0.2.0"
git tag -s v0.2.0 -m "Folderol 0.2.0"
```

### Build artifacts

```bash
cargo build --release -p folderol-cli -p folderol-gui

tar -czf folderol-0.2.0-x86_64-linux.tar.gz \
  -C target/release folderol folderol-gui \
  -C "$PWD" LICENSE README.md data/

sha256sum folderol-0.2.0-x86_64-linux.tar.gz \
  > folderol-0.2.0-x86_64-linux.tar.gz.sha256
```

The tarball contains binaries, desktop file, license and README. It does not
contain a model; models are downloaded explicitly or packaged separately with
their license.

### Verify on a clean machine

- [ ] Install the tarball into `/usr/local` manually; launch from a fresh user.
- [ ] Install the Arch package in a clean VM.
- [ ] Install the Flatpak in a clean VM with no home directory.
- [ ] Run the release smoke list in `TESTING.md` §13.
- [ ] Confirm the installed binary reports the right version:
      `folderol --version`, `folderol-gui --version`.

### Publish

- [ ] Push the tag: `git push origin main --tags`.
- [ ] Create the GitHub release with notes, tarball, checksum.
- [ ] Update AUR `folderol` and `folderol-git`.
- [ ] Submit the Flatpak update to Flathub.
- [ ] Update the website download links and version number.
- [ ] Announce in the places the project actually lives; no spam.

### After

- [ ] Watch the issue tracker for 72 hours.
- [ ] Verify the package installs for at least one person who is not the
      maintainer.
- [ ] Record any hotfix in the changelog, not just in git history.
- [ ] Open the next milestone with the lessons from this one.

## 5. Packaging details

### Arch (AUR)

`packaging/PKGBUILD` builds `folderol-git`:

```bash
pkgname=folderol-git
pkgver=0.2.0
depends=(gtk4 libadwaita sqlite)
makedepends=(cargo rust)
source=("git+https://github.com/SamiulH25/folderol.git")
```

Install:

```text
/usr/bin/folderol
/usr/bin/folderol-gui
/usr/share/applications/io.github.samiulh25.Folderol.desktop
/usr/share/metainfo/io.github.samiulh25.Folderol.metainfo.xml
/usr/share/icons/hicolor/scalable/apps/io.github.samiulh25.Folderol.svg
/usr/share/licenses/folderol/LICENSE
```

A stable `folderol` package uses the release tarball and its checksum.

### Flatpak

- Runtime: GNOME 48 or newer.
- Finish args: `--share=ipc`, `--socket=wayland`, `--socket=fallback-x11`,
  `--device=dri`, `--filesystem=home`, `--talk-name=org.freedesktop.portal.*`
  as required by the runtime.
- **No network permission.** The model download is a host-side action or a
  separately permissioned optional build until a portal-based flow exists.
- `cargo-sources.json` is generated with `flatpak-cargo-generator.py` and
  committed.
- The app ID matches the desktop file and AppStream component.

### Desktop entry

```ini
[Desktop Entry]
Type=Application
Name=Folderol
GenericName=File Manager
Comment=Browse, search and organise files with local learning
Exec=folderol-gui %U
Icon=io.github.samiulh25.Folderol
Terminal=false
Categories=System;FileManager;GTK;
MimeType=inode/directory;
StartupNotify=true
Actions=new-window;search;

[Desktop Action new-window]
Name=New Window
Exec=folderol-gui --new-window

[Desktop Action search]
Name=Search Files
Exec=folderol-gui --search
```

Registering `inode/directory` makes Folderol selectable as the default file
manager; it never claims the association silently.

### AppStream

Required fields: `id`, `name`, `summary`, `description`, `launchable`,
`provides`, `releases`, `content_rating`, `developer_name`, `url`, `license`.
Screenshots are added when the UI stabilises; AppStream validation
(`appstreamcli validate`) is a CI job.

## 6. Changelog

Keep a human changelog at the repository root. Format:

```markdown
## [0.2.0] - 2026-11-XX

### Added
- Ignore-aware scanning with `.folderolignore` and default excludes.
- `folderol watch` for live index updates.

### Changed
- Scans skip unchanged files; second runs are near-instant.
- Catalog upgraded to schema 2 (meta table); automatic migration.

### Fixed
- Human labels survive rescans.
- Catalog file is created with mode 0600.

### Security
- Process umask hardened at startup; index directory is private.
```

Link the comparison range at the bottom. Do not list every commit.

## 7. Signing and verification

- Git tags are signed with the maintainer’s key.
- Release tarballs are accompanied by SHA-256; a future release adds a detached
  GPG signature.
- Flatpak is signed by Flathub’s infrastructure.
- The AppStream metadata and desktop file are validated before publishing.
- Users can verify the source tag and rebuild; the build is reproducible enough
  to compare binaries when the toolchain matches, and this is documented as a
  goal rather than a guarantee until M5.

## 8. Rollback and security releases

- A broken stable release is superseded, not deleted. Yanking a Flatpak build is
  allowed when it is unsafe; the version number is never reused.
- A security fix ships as a `PATCH` on the oldest supported minor, with a
  clear advisory, affected versions and a workaround.
- Supported versions: the latest minor and the previous minor for 90 days after
  a stable release. Before 1.0, only the latest minor.
- The database schema never downgrades. A user who opens a newer catalog with
  an older binary gets a clear refusal, not corruption.

## 9. Post-release

Within one week:

- Triage every report labelled `regression`.
- Record install failures by distro and packaging channel.
- Update the roadmap with what slipped and why.
- Archive benchmark numbers under `docs/benchmarks/YYYY-MM-DD.md`.
- If the release required a hotfix, write a short post-mortem in the issue
  tracker and link it from the changelog.

## 10. Release notes template

```markdown
# Folderol 0.2.0

A file browser that learns where your files belong. This release makes the
index trustworthy: ignore rules, live updates, private permissions and a
benchmark gate.

## Highlights
- …
- …

## Upgrade notes
- The catalog migrates automatically from schema 1.
- Rescan once if you changed an ignore file before this release.

## Performance
- 100k files: scan in X s, search p95 Y ms.

## Known issues
- …

## Thanks
- …
```
