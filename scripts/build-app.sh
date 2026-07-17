#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP="$ROOT/dist/iMessage Browser.app"

cd "$ROOT"
cargo build --release --bin imessage-gui

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp "$ROOT/target/release/imessage-gui" "$APP/Contents/MacOS/imessage-gui"
cp "$ROOT/macos/Info.plist" "$APP/Contents/Info.plist"

echo "Built unsigned app: $APP"
