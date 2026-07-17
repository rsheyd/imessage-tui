# AGENTS.md

Standalone Rust TUI for browsing and exporting macOS Messages data.

## Instructions

- Keep database access read-only.
- Treat `imessage-database` as the source of truth for decoding modern message bodies.
- Preserve exact time-range behavior for exports.
- Complete the manual acceptance checklist in `PROJECT-STATUS.md` before calling a phase complete.

## File map

- `README.md`: Stable overview, usage, keys, privacy, and limitations.
- `CHANGELOG.md`: User-facing changes organized by release version.
- `CONTRIBUTING.md`: Development guidance, including version bumps and local reinstalls.
- `LICENSE`: MIT license for the project.
- `PROJECT-STATUS.md`: Current status and manual acceptance checklist.
- `macos/Info.plist`: Metadata for the unsigned GUI app bundle.
- `scripts/build-app.sh`: Builds and assembles `dist/iMessage Browser.app`.
- `.github/workflows/ci.yml`: macOS formatting, test, and Clippy checks.
- `assets/screenshot.png`: Anonymized TUI screenshot used in the README.
- `assets/messages-screenshot.png`: Anonymized conversation screenshot used in the README.
- `assets/export-range-screenshot.png`: Anonymized export-menu screenshot used in the README.
- `src/app.rs`: Application state, navigation, paging, and export workflow.
- `src/db.rs`: Read-only Messages/Contacts access and message decoding.
- `src/export.rs`: Markdown generation and safe filenames.
- `src/model.rs`: Shared conversation, message, and export-range models.
- `src/ui.rs`: Ratatui rendering.
- `src/main.rs`: Terminal lifecycle and event loop.
- `src/lib.rs`: Shared database, export, and model modules used by both interfaces.
- `src/bin/imessage-gui.rs`: egui conversation browser and exporter prototype.
