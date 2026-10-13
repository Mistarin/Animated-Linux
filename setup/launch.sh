#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RELEASE_BIN="$ROOT_DIR/src-tauri/target/release/aetherpaper"
DEBUG_BIN="$ROOT_DIR/src-tauri/target/debug/aetherpaper"

echo "================================================================"
echo "          AetherPaper — Git Edition Launcher                    "
echo "================================================================"

# Locate libsteam_api.so from cargo build artifacts
STEAM_LIB="$(find "$ROOT_DIR/src-tauri/target" -name "libsteam_api.so" 2>/dev/null | head -n 1)"
if [ -n "$STEAM_LIB" ]; then
    STEAM_DIR="$(dirname "$STEAM_LIB")"
    # Ensure it is copied alongside binaries
    [ -d "$(dirname "$RELEASE_BIN")" ] && cp -u "$STEAM_LIB" "$(dirname "$RELEASE_BIN")/" 2>/dev/null || true
    [ -d "$(dirname "$DEBUG_BIN")" ] && cp -u "$STEAM_LIB" "$(dirname "$DEBUG_BIN")/" 2>/dev/null || true
    [ -d "$ROOT_DIR/dist" ] && cp -u "$STEAM_LIB" "$ROOT_DIR/dist/" 2>/dev/null || true
    export LD_LIBRARY_PATH="$(dirname "$RELEASE_BIN"):$(dirname "$DEBUG_BIN"):$STEAM_DIR:${LD_LIBRARY_PATH:-}"
fi

# Check for --dev flag
USE_DEV=0
for arg in "$@"; do
    if [ "$arg" = "--dev" ]; then
        USE_DEV=1
    fi
done

# Check essential runtime dependencies
if ! command -v ffmpeg >/dev/null 2>&1; then
    echo "[Warning] ffmpeg not found in PATH. Video thumbnail generation may fail."
fi

# 1. Dev mode requested
if [ "$USE_DEV" -eq 1 ]; then
    echo "[Launcher] Launching in Development mode (Live HMR)..."
    exec pnpm tauri dev
fi

# 2. Release binary exists
if [ -f "$RELEASE_BIN" ]; then
    echo "[Launcher] Starting optimized production release binary..."
    exec "$RELEASE_BIN" "$@"
fi

# 3. Debug binary exists
if [ -f "$DEBUG_BIN" ]; then
    echo "[Launcher] Release binary not found. Launching debug binary..."
    exec "$DEBUG_BIN" "$@"
fi

# 4. Fallback: Build and run
echo "[Launcher] No binary found. Compiling release build now..."
bash "$ROOT_DIR/setup/build_image.sh" --binary-only
exec "$RELEASE_BIN" "$@"
