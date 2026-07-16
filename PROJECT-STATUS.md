# Project status

## Current phase

V1 is implemented. Automated validation passes, and live acceptance against the user's Messages database was completed successfully on July 16, 2026.

## Automated validation

Freshly rerun on July 16, 2026:

- [x] `cargo fmt --check`
- [x] `cargo test` (including fixture-backed conversation and paging test)
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] Missing Full Disk Access produces the inherited actionable error.

## V1 acceptance checklist

- [x] Conversations appear newest first.
- [x] Contact names resolve where available.
- [ ] Conversation search filters by contact name and phone number; `Esc` restores the full list.
- [x] Enter opens the selected conversation at the newest message.
- [x] Arrow keys and Page Up/Down navigate messages and load older pages.
- [x] Each export range creates readable Markdown in the selected path.
- [x] The default export path is the launch directory.
- [x] `imessage-tui` is installed at `~/.cargo/bin/imessage-tui` and starts the binary.

## Deferred to V2

- Attachment copying.
- Additional export formats.
