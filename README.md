# imessage-tui

A local, read-only terminal browser and Markdown exporter for the macOS Messages database.

<p align="center">
  <img src="assets/conversation-list-screenshot.png" alt="imessage-tui showing the recent conversations screen" width="720">
</p>

**Table of Contents**
- [imessage-tui](#imessage-tui)
  - [V1 features](#v1-features)
  - [Requirements](#requirements)
  - [Install from source](#install-from-source)
  - [Build without installing](#build-without-installing)
  - [GUI prototype](#gui-prototype)
  - [Keys](#keys)
    - [Conversations](#conversations)
    - [Messages](#messages)
  - [More screenshots](#more-screenshots)
  - [Privacy](#privacy)
  - [V1 limitations](#v1-limitations)
  - [License](#license)

## V1 features

- Lists recent conversations newest first.
- Resolves contact names from the local macOS Contacts database when possible.
- Searches conversations by contact name or phone number.
- Opens a conversation at its latest 20 messages and pages older messages on demand.
- Exports one conversation to Markdown for the last hour, last 24 hours, a custom number of hours or days, or all time.
- Defaults export paths to the directory where `imessage-tui` was started.
- Preserves message timestamps and sender names, and includes reactions and attachment placeholders.

## Requirements

- macOS with Messages data stored locally.
- Full Disk Access for the terminal application that launches `imessage-tui`.
- Rust is required only when building from source.

## Install from source

Rust is required to install from source. After cloning this repository, run:

```sh
cargo install --path .
```

This builds and installs `imessage-tui` in Cargo's binary directory, which is
usually `~/.cargo/bin`. Start it with:

```sh
imessage-tui
```

## Build without installing

```sh
cargo build --release
./target/release/imessage-tui
```

## GUI prototype

An unsigned Apple Silicon macOS app prototype is available alongside the TUI.
Build and open it with:

```sh
./scripts/build-app.sh
open "dist/iMessage Browser.app"
```

The GUI provides conversation search, message browsing and paging, and the same
Markdown export ranges as the TUI. Exports default to the current user's
Documents folder.

The app needs Full Disk Access to read the local Messages database. If it shows
the permission screen, open **System Settings → Privacy & Security → Full Disk
Access**, add `dist/iMessage Browser.app`, enable it, then quit and reopen the app.
Because this prototype is unsigned, it is intended for local testing rather
than redistribution.

## Keys

### Conversations

- `↑` / `↓` or `j` / `k`: move
- `Page Up` / `Page Down`: move faster
- `Enter`: open conversation
- `/`: search by contact name or phone number; `Esc` clears the search
- `q`: quit

### Messages

- `↑` / `↓` or `j` / `k`: older/newer message
- `Page Up` / `Page Down`: jump ten messages
- `Home` / `End`: oldest/latest loaded message
- `e`: export
- `q`, `Esc`, or `Backspace`: return to conversations

## More screenshots

<p align="center">
  <img src="assets/messages-screenshot.png" alt="imessage-tui showing messages in a conversation" width="49%">
  <img src="assets/export-range-screenshot.png" alt="imessage-tui showing the export range menu" width="49%">
</p>

## Privacy

The Messages and Contacts databases are opened read-only. Exports are ordinary unencrypted Markdown files, so store them appropriately.

## V1 limitations

- Attachment files are not copied. Markdown contains placeholders.

## License

MIT. See [LICENSE](LICENSE).
