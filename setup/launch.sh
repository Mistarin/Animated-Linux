#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

APP_BIN="animated-linux"
RELEASE_BIN="$ROOT_DIR/src-tauri/target/release/$APP_BIN"
DEBUG_BIN="$ROOT_DIR/src-tauri/target/debug/$APP_BIN"

echo "================================================================"
echo "          Animated Linux Launcher                               "
echo "================================================================"

# Locate libsteam_api.so from cargo build artifacts if present
STEAM_LIB="$(find "$ROOT_DIR/src-tauri/target" -name "libsteam_api.so" 2>/dev/null | head -n 1 || true)"
if [ -n "$STEAM_LIB" ]; then
    STEAM_DIR="$(dirname "$STEAM_LIB")"
    [ -d "$(dirname "$RELEASE_BIN")" ] && cp -u "$STEAM_LIB" "$(dirname "$RELEASE_BIN")/" 2>/dev/null || true
    [ -d "$(dirname "$DEBUG_BIN")" ] && cp -u "$STEAM_LIB" "$(dirname "$DEBUG_BIN")/" 2>/dev/null || true
    [ -d "$ROOT_DIR/dist" ] && cp -u "$STEAM_LIB" "$ROOT_DIR/dist/" 2>/dev/null || true
    export LD_LIBRARY_PATH="$(dirname "$RELEASE_BIN"):$(dirname "$DEBUG_BIN"):$STEAM_DIR:${LD_LIBRARY_PATH:-}"
fi

# Check for flags
USE_DEV=0
FORCE_BUILD=0
for arg in "$@"; do
    if [ "$arg" = "--dev" ]; then
        USE_DEV=1
    elif [ "$arg" = "--build" ]; then
        FORCE_BUILD=1
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

# 2. Check if binary is missing or source code is newer than existing binary
NEED_BUILD=0
if [ ! -f "$RELEASE_BIN" ] || [ "$FORCE_BUILD" -eq 1 ]; then
    NEED_BUILD=1
else
    # Find any modified source file newer than the release binary
    NEWER_SRC="$(find "$ROOT_DIR/src" "$ROOT_DIR/src-tauri/src" "$ROOT_DIR/index.html" "$ROOT_DIR/src-tauri/tauri.conf.json" -type f -newer "$RELEASE_BIN" 2>/dev/null | head -n 1 || true)"
    if [ -n "$NEWER_SRC" ]; then
        echo "[Launcher] Source changes detected newer than binary ($NEWER_SRC)."
        echo "[Launcher] Auto-rebuilding fresh binary to ensure changes take effect..."
        NEED_BUILD=1
    fi
fi

if [ "$NEED_BUILD" -eq 1 ]; then
    bash "$ROOT_DIR/setup/build_image.sh" --binary-only
fi

# 3. Launch release binary
if [ -f "$RELEASE_BIN" ]; then
    echo "[Launcher] Starting optimized production release binary ($APP_BIN)..."
    exec "$RELEASE_BIN" "$@"
fi

# 4. Fallback debug binary
if [ -f "$DEBUG_BIN" ]; then
    echo "[Launcher] Starting debug binary ($APP_BIN)..."
    exec "$DEBUG_BIN" "$@"
fi

echo "[Error] Failed to build or find binary."
exit 1
