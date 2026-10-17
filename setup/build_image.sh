#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "================================================================"
echo "          Building Animated Linux Native Executable             "
echo "================================================================"

mkdir -p "$ROOT_DIR/dist"

echo "[1/1] Building Frontend & Release Binary with Embedded Assets..."
pnpm tauri build --no-bundle

cp "$ROOT_DIR/src-tauri/target/release/animated-linux" "$ROOT_DIR/dist/animated-linux-x86_64"
echo "[OK] Standalone executable built: dist/animated-linux-x86_64"

echo ""
echo "================================================================"
echo "[OK] Done! Output available at: $ROOT_DIR/dist/animated-linux-x86_64"
echo "================================================================"
ls -lh "$ROOT_DIR/dist"
