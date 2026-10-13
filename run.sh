#!/usr/bin/env bash
# Copyright (C) 2026 Animated Linux Contributors
# Licensed under GNU GPL-3.0

set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

if [ "$1" == "--dev" ]; then
    echo "[Animated Linux] Launching dev environment with Vite hot module reloading..."
    exec pnpm tauri dev
else
    if [ ! -f "dist/animated-linux-x86_64" ]; then
        echo "[Animated Linux] Standalone binary missing. Compiling release build..."
        ./setup/build_image.sh --binary-only
    fi
    exec ./dist/animated-linux-x86_64 "$@"
fi
