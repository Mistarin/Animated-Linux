#!/usr/bin/env bash
# Copyright (C) 2026 Animated Linux Contributors
# Licensed under GNU GPL-3.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

APP_BIN="animated-linux"
RELEASE_BIN="$ROOT_DIR/src-tauri/target/release/$APP_BIN"
TARGET_BIN="$ROOT_DIR/dist/animated-linux-x86_64"

# Force GTK and WebKitGTK to prefer dark theme
export GTK_THEME="Adwaita:dark"
export GTK_APPLICATION_PREFER_DARK_THEME="1"

echo "================================================================"
echo "          Animated Linux Launcher                               "
echo "================================================================"

# Ensure KDE Plasma MPV plugin environment if running under Plasma
if [ "${XDG_CURRENT_DESKTOP:-}" = "KDE" ] || command -v qdbus >/dev/null 2>&1; then
    bash "$ROOT_DIR/setup/install_plasma_mpv.sh" >/dev/null 2>&1 || true
fi

# Check essential runtime dependencies
if ! command -v ffmpeg >/dev/null 2>&1; then
    echo "[Warning] ffmpeg not found in PATH. Video thumbnail generation may fail."
fi

# Check for flags
USE_RELEASE=0
FORCE_BUILD=0
for arg in "$@"; do
    if [ "$arg" = "--release" ] || [ "$arg" = "--prod" ] || [ "$arg" = "--binary" ]; then
        USE_RELEASE=1
    elif [ "$arg" = "--build" ]; then
        FORCE_BUILD=1
        USE_RELEASE=1
    elif [ "$arg" = "-h" ] || [ "$arg" = "--help" ]; then
        echo "Usage: ./setup/launch.sh [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  (default)     Run directly from source code via 'pnpm tauri dev' (Live HMR)"
        echo "  --release     Run precompiled standalone production binary"
        echo "  --build       Force rebuild of standalone production binary"
        echo "  -h, --help    Show this help message"
        exit 0
    fi
done

cleanup_port() {
    local port="$1"
    local pids=""
    if command -v lsof >/dev/null 2>&1; then
        pids="$(lsof -ti :"$port" 2>/dev/null || true)"
    elif command -v fuser >/dev/null 2>&1; then
        pids="$(fuser "$port"/tcp 2>/dev/null || true)"
    fi

    if [ -n "$pids" ]; then
        echo "[Launcher] Port $port is already in use by PID(s): $pids"
        echo "[Launcher] Terminating stale process on port $port..."
        kill -15 $pids 2>/dev/null || true
        sleep 0.5
        local remaining=""
        if command -v lsof >/dev/null 2>&1; then
            remaining="$(lsof -ti :"$port" 2>/dev/null || true)"
        fi
        if [ -n "$remaining" ]; then
            kill -9 $remaining 2>/dev/null || true
            sleep 0.2
        fi
        echo "[Launcher] Port $port is now free."
    fi
}

# 1. Default: Run directly from source code with Live HMR & incremental compile
if [ "$USE_RELEASE" -eq 0 ]; then
    cleanup_port 1420
    if [ ! -d "$ROOT_DIR/node_modules" ] || [ ! -f "$ROOT_DIR/node_modules/@tauri-apps/cli/package.json" ]; then
        echo "[Launcher] Installing dependencies via pnpm..."
        pnpm install
    fi
    echo "[Launcher] Running directly from source code (Live HMR)..."
    exec pnpm tauri dev
fi

# 2. Release mode requested: Check if binary is missing or source code has changed
NEED_BUILD=0
HASH_FILE="$ROOT_DIR/.last_build_hash"

compute_source_hash() {
    if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        (git rev-parse HEAD 2>/dev/null && git status --porcelain 2>/dev/null) | sha256sum | awk '{print $1}'
    else
        find "$ROOT_DIR/src" "$ROOT_DIR/src-tauri/src" "$ROOT_DIR/src-tauri/tauri.conf.json" "$ROOT_DIR/index.html" -type f -exec sha256sum {} + 2>/dev/null | sort | sha256sum | awk '{print $1}'
    fi
}

CURRENT_HASH="$(compute_source_hash)"

if [ ! -f "$RELEASE_BIN" ] || [ ! -f "$TARGET_BIN" ] || [ "$FORCE_BUILD" -eq 1 ]; then
    NEED_BUILD=1
elif [ ! -f "$HASH_FILE" ] || [ "$(cat "$HASH_FILE" 2>/dev/null)" != "$CURRENT_HASH" ]; then
    echo "[Launcher] Source changes detected. Rebuilding production release binary..."
    NEED_BUILD=1
fi

if [ "$NEED_BUILD" -eq 1 ]; then
    bash "$ROOT_DIR/setup/build_image.sh" --binary-only
    echo "$CURRENT_HASH" > "$HASH_FILE"
fi

if [ ! -f "$TARGET_BIN" ]; then
    TARGET_BIN="$RELEASE_BIN"
fi

if [ -f "$TARGET_BIN" ]; then
    echo "[Launcher] Starting optimized production release binary ($APP_BIN)..."
    exec "$TARGET_BIN" "$@"
else
    echo "[Error] Failed to build or find release binary."
    exit 1
fi
