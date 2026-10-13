#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

APP_BIN="animated-linux"
RELEASE_BIN="$ROOT_DIR/src-tauri/target/release/$APP_BIN"
DEBUG_BIN="$ROOT_DIR/src-tauri/target/debug/$APP_BIN"

# Force GTK and WebKitGTK to prefer dark theme
export GTK_THEME="Adwaita:dark"
export GTK_APPLICATION_PREFER_DARK_THEME="1"

echo "================================================================"
echo "          Animated Linux Launcher                               "
echo "================================================================"

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
elif [ -f "$ROOT_DIR/dist/animated-linux-x86_64" ]; then
    echo "[Launcher] Starting standalone release binary from dist..."
    exec "$ROOT_DIR/dist/animated-linux-x86_64" "$@"
else
    echo "[Error] Failed to build or find release binary."
    exit 1
fi
