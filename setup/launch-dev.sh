#!/usr/bin/env bash
# Copyright (C) 2026 Animated Linux Contributors
# Licensed under GNU GPL-3.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Force GTK and WebKitGTK to prefer dark theme
export GTK_THEME="Adwaita:dark"
export GTK_APPLICATION_PREFER_DARK_THEME="1"

echo "================================================================"
echo "          Animated Linux - Development Launcher (Live HMR)     "
echo "================================================================"

# Ensure KDE Plasma MPV plugin environment if running under Plasma
if [ "${XDG_CURRENT_DESKTOP:-}" = "KDE" ] || command -v qdbus >/dev/null 2>&1; then
    bash "$ROOT_DIR/setup/install_plasma_mpv.sh" >/dev/null 2>&1 || true
fi

# Check essential runtime dependencies
if ! command -v ffmpeg >/dev/null 2>&1; then
    echo "[Warning] ffmpeg not found in PATH. Video thumbnail generation may fail."
fi

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

cleanup_port 1420

# Ensure dependencies are installed
if [ ! -d "$ROOT_DIR/node_modules" ] || [ ! -f "$ROOT_DIR/node_modules/@tauri-apps/cli/package.json" ]; then
    echo "[Launcher] Dependencies not installed. Running pnpm install..."
    pnpm install
fi

echo "[Launcher] Starting Tauri development server with Live HMR..."
exec pnpm tauri dev "$@"
