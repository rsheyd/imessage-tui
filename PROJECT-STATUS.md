# Project status

## Current phase

Version 0.1.2 is ready for source-only publication. Automated validation passes, and live TUI acceptance against the user's Messages database was completed successfully on July 16, 2026. The GUI remains an unsigned Apple Silicon prototype with incomplete manual acceptance.

## Automated validation

Formatting, tests, and Clippy were freshly rerun on August 24, 2026. Missing Full Disk Access behavior was last accepted on July 16, 2026.

- [x] `cargo fmt --check`
- [x] `cargo test` (12 tests, including fixture-backed conversation, paging, export-path, and directory-creation coverage)
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] Missing Full Disk Access produces the inherited actionable error.

## V1 acceptance checklist

- [x] Conversations appear newest first.
- [x] Contact names resolve where available.
- [ ] Conversation search filters by contact name and phone number; `Esc` restores the full list.
- [x] Enter opens the selected conversation at the newest message.
- [x] Arrow keys and Page Up/Down navigate messages and load older pages.
- [x] Each export range creates readable Markdown in the selected path.
- [ ] The default export path is an `exports/` directory beneath the launch directory and missing directories are created automatically.
- [x] `imessage-tui` is installed at `~/.cargo/bin/imessage-tui` and starts the binary.

## Deferred to V2

- Attachment copying.
- Additional export formats.

## GUI prototype

The egui prototype shares the read-only database, search, message decoding, and
Markdown export code with the TUI. It builds as an unsigned Apple Silicon app
at `dist/iMessage Browser.app`.

### Manual acceptance checklist

- [x] The app launches from Finder or with `open "dist/iMessage Browser.app"`.
- [x] Full Disk Access guidance appears when access is unavailable.
- [ ] Conversations appear newest first and search matches names and formatted phone numbers.
- [ ] Selecting a conversation shows its latest messages and **Load older messages** prepends another page.
- [ ] Each export range writes readable Markdown to the chosen path.
- [ ] The existing TUI still launches and completes its accepted workflows.
