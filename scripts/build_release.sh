#!/usr/bin/env bash
set -e

echo "=== Building AetherPaper for Steam Distribution ==="

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

echo "Building Frontend & Release Binary with Embedded Assets..."
pnpm tauri build --no-bundle

mkdir -p "$DIR/dist"
if [ -f "$DIR/src-tauri/target/release/animated-linux" ]; then
    cp -f "$DIR/src-tauri/target/release/animated-linux" "$DIR/dist/animated-linux-x86_64"
fi

# Ensure libsteam_api.so is located alongside binary
STEAM_SO="$(find "$DIR/src-tauri/target" -name "libsteam_api.so" 2>/dev/null | head -n 1)"
if [ -n "$STEAM_SO" ]; then
    cp -u "$STEAM_SO" "$DIR/src-tauri/target/release/"
    cp -u "$STEAM_SO" "$DIR/dist/"
fi

echo "=== Build Complete! ==="
echo "Release binary location: $DIR/dist/animated-linux-x86_64"
