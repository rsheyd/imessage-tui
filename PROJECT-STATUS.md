# Project status

## Current phase

Version 0.1.2 was published as a source-only GitHub release on August 24, 2026. Version 0.1.3 is in development with image copying and availability counts for Markdown exports. A live custom-hour TUI export passed after local image downloads, with no unavailable image attachments; other range and GUI acceptance checks remain open. Missing image source files remain placeholders and are counted as unavailable. The GUI remains an unsigned Apple Silicon prototype with incomplete manual acceptance.

## Automated validation

Formatting, tests, and Clippy were rerun on September 25, 2026. The TUI binary was reinstalled and the unsigned GUI app bundle was rebuilt. Missing Full Disk Access behavior was last accepted on July 16, 2026.

- [x] `cargo fmt --check`
- [x] `cargo test` (13 tests, including fixture-backed conversation, paging, export-path, directory-creation, and image-copying coverage)
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] Missing Full Disk Access produces the inherited actionable error.

## V1 acceptance checklist

- [x] Conversations appear newest first.
- [x] Contact names resolve where available.
- [ ] Conversation search filters by contact name and phone number; `Esc` restores the full list.
- [x] Enter opens the selected conversation at the newest message.
- [x] Arrow keys and Page Up/Down navigate messages and load older pages.
- [x] Each export range creates readable Markdown in the selected path.
- [x] A live custom-hour TUI export contains only messages in range and resolves every copied image link.
- [x] The export header reports copied and unavailable image attachment counts, including a verified zero-unavailable live export.
- [ ] Each export range copies only images attached to selected messages, and Markdown links open the copied files.
- [ ] Missing image files and non-image attachments remain visible as placeholders.
- [ ] The default export path is an `exports/` directory beneath the launch directory and missing directories are created automatically.
- [x] `imessage-tui` is installed at `~/.cargo/bin/imessage-tui` and starts the binary.

## Deferred to V2

- Copying non-image attachments.
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
- [ ] Image attachments in the selected range are copied and linked from the exported Markdown.
- [ ] The existing TUI still launches and completes its accepted workflows.
