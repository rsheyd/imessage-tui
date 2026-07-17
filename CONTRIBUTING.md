# Contributing

**Table of Contents**
- [Contributing](#contributing)
  - [Bumping the version and reinstalling](#bumping-the-version-and-reinstalling)
  - [Publishing a GitHub release](#publishing-a-github-release)

## Bumping the version and reinstalling

For a user-facing change, such as adding conversation search, increment the
package version in `Cargo.toml` and describe the change in `CHANGELOG.md`. For
example:

```toml
version = "0.1.1"
```

Then update `Cargo.lock`, validate the project, and reinstall the binary:

```sh
cargo check
cargo install --path . --force
```

`cargo check` updates the package version recorded in `Cargo.lock`. Confirm the
installed version with:

```sh
cargo install --list | rg '^imessage-tui v0.1.1:'
```

If the binary later gains a `--version` option, it can also be checked with
`imessage-tui --version`.

## Publishing a GitHub release

After the version bump and related changes are committed and pushed:

1. Open the repository's **Releases** page and choose **Draft a new release**.
2. Create a tag matching the package version, such as `v0.1.1`, targeting
   `main`.
3. Use a title such as `imessage-tui 0.1.1` and summarize the release using
   `CHANGELOG.md`.
4. Leave the release label set to **None** for a production-ready release. Use
   **Pre-release** only for unfinished test versions such as betas.
5. Publish the release. GitHub automatically attaches source-code archives.

Users with Rust can install a tagged release with:

```sh
cargo install --git https://github.com/rsheyd/imessage-tui.git --tag v0.1.1
```

## Building the unsigned macOS app

On Apple Silicon macOS, build the local GUI prototype with:

```sh
./scripts/build-app.sh
open "dist/iMessage Browser.app"
```

Keep the version strings in `macos/Info.plist` aligned with `Cargo.toml` when
bumping the package version.
