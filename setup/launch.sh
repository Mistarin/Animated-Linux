#!/usr/bin/env bash
# Copyright (C) 2026 Animated Linux Contributors
# Licensed under GNU GPL-3.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

APP_NAME="animated-linux"
DIST_DIR="$ROOT_DIR/dist"
APPIMAGE_BIN="$DIST_DIR/Animated_Linux-x86_64.AppImage"
USER_APPIMAGE="$HOME/Applications/Animated_Linux-x86_64.AppImage"
RELEASE_EXE="$DIST_DIR/animated-linux-x86_64"
CARGO_RELEASE_EXE="$ROOT_DIR/src-tauri/target/release/$APP_NAME"

# Force GTK and WebKitGTK to prefer dark theme
export GTK_THEME="Adwaita:dark"
export GTK_APPLICATION_PREFER_DARK_THEME="1"

print_header() {
    echo "================================================================"
    echo "          Animated Linux Launcher                               "
    echo "================================================================"
}

show_help() {
    print_header
    echo "Usage: ./setup/launch.sh [OPTIONS] [-- APP_ARGS...]"
    echo ""
    echo "Priority & Availability Launching:"
    echo "  By default, launches available targets in order of priority:"
    echo "    1. Standalone AppImage (dist/Animated_Linux-x86_64.AppImage)"
    echo "    2. Standalone Native Binary (dist/animated-linux-x86_64)"
    echo "    3. Auto-builds release binary if no build exists yet"
    echo ""
    echo "Options:"
    echo "  --appimage        Force launching the standalone AppImage package"
    echo "  --bin, --binary   Force launching the native compiled binary"
    echo "  --dev             Launch directly from source code with Live HMR (launch-dev.sh)"
    echo "  --build           Force rebuild before launching"
    echo "  -h, --help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./run.sh                  # Launch preferred available build (AppImage -> binary)"
    echo "  ./run.sh --dev            # Run development server with live reload"
    echo "  ./run.sh --bin            # Force running the native release executable"
    echo "  ./run.sh --build          # Rebuild and run latest release"
    exit 0
}

# Ensure KDE Plasma MPV plugin environment if running under Plasma
if [ "${XDG_CURRENT_DESKTOP:-}" = "KDE" ] || command -v qdbus >/dev/null 2>&1; then
    bash "$ROOT_DIR/setup/install_plasma_mpv.sh" >/dev/null 2>&1 || true
fi

# Check essential runtime dependencies
if ! command -v ffmpeg >/dev/null 2>&1; then
    echo "[Warning] ffmpeg not found in PATH. Video thumbnail generation may fail."
fi

# Parse launcher options
TARGET_MODE="auto" # auto | appimage | binary | dev
FORCE_BUILD=0
PASSTHROUGH_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dev)
            TARGET_MODE="dev"
            shift
            ;;
        --appimage)
            TARGET_MODE="appimage"
            shift
            ;;
        --bin|--binary|--release|--prod)
            TARGET_MODE="binary"
            shift
            ;;
        --build)
            FORCE_BUILD=1
            shift
            ;;
        -h|--help)
            show_help
            ;;
        --)
            shift
            PASSTHROUGH_ARGS+=("$@")
            break
            ;;
        *)
            PASSTHROUGH_ARGS+=("$1")
            shift
            ;;
    esac
done

# If development mode requested, delegate directly to launch-dev.sh
if [ "$TARGET_MODE" = "dev" ]; then
    exec "$ROOT_DIR/setup/launch-dev.sh" "${PASSTHROUGH_ARGS[@]}"
fi

print_header

# Handle forced build
if [ "$FORCE_BUILD" -eq 1 ]; then
    echo "[Launcher] Force rebuild requested. Running build script..."
    if [ "$TARGET_MODE" = "appimage" ]; then
        bash "$ROOT_DIR/setup/build_image.sh" --bundle
    else
        bash "$ROOT_DIR/setup/build_image.sh" --binary-only
    fi
fi

# Locate existing targets based on priority and availability
FOUND_APPIMAGE=""
if [ -f "$APPIMAGE_BIN" ] && [ -x "$APPIMAGE_BIN" ]; then
    FOUND_APPIMAGE="$APPIMAGE_BIN"
elif [ -f "$USER_APPIMAGE" ] && [ -x "$USER_APPIMAGE" ]; then
    FOUND_APPIMAGE="$USER_APPIMAGE"
fi

FOUND_BINARY=""
if [ -f "$RELEASE_EXE" ] && [ -x "$RELEASE_EXE" ]; then
    FOUND_BINARY="$RELEASE_EXE"
elif [ -f "$CARGO_RELEASE_EXE" ] && [ -x "$CARGO_RELEASE_EXE" ]; then
    FOUND_BINARY="$CARGO_RELEASE_EXE"
fi

SELECTED_EXEC=""
EXEC_TYPE=""

if [ "$TARGET_MODE" = "appimage" ]; then
    if [ -z "$FOUND_APPIMAGE" ]; then
        echo "[Launcher] AppImage requested but not found. Building AppImage..."
        bash "$ROOT_DIR/setup/build_image.sh" --bundle
        if [ -f "$APPIMAGE_BIN" ]; then
            FOUND_APPIMAGE="$APPIMAGE_BIN"
        fi
    fi
    SELECTED_EXEC="$FOUND_APPIMAGE"
    EXEC_TYPE="AppImage package"
elif [ "$TARGET_MODE" = "binary" ]; then
    if [ -z "$FOUND_BINARY" ]; then
        echo "[Launcher] Native executable requested but not found. Building release binary..."
        bash "$ROOT_DIR/setup/build_image.sh" --binary-only
        if [ -f "$RELEASE_EXE" ]; then
            FOUND_BINARY="$RELEASE_EXE"
        elif [ -f "$CARGO_RELEASE_EXE" ]; then
            FOUND_BINARY="$CARGO_RELEASE_EXE"
        fi
    fi
    SELECTED_EXEC="$FOUND_BINARY"
    EXEC_TYPE="Native binary"
else
    # Auto Mode: Priority 1: AppImage, Priority 2: Native Executable
    if [ -n "$FOUND_APPIMAGE" ]; then
        SELECTED_EXEC="$FOUND_APPIMAGE"
        EXEC_TYPE="AppImage package (Priority 1)"
    elif [ -n "$FOUND_BINARY" ]; then
        SELECTED_EXEC="$FOUND_BINARY"
        EXEC_TYPE="Native binary (Priority 2)"
    else
        echo "[Launcher] No pre-built AppImage or executable found."
        echo "[Launcher] Auto-building standalone release binary..."
        bash "$ROOT_DIR/setup/build_image.sh" --binary-only
        if [ -f "$RELEASE_EXE" ]; then
            SELECTED_EXEC="$RELEASE_EXE"
        elif [ -f "$CARGO_RELEASE_EXE" ]; then
            SELECTED_EXEC="$CARGO_RELEASE_EXE"
        fi
        EXEC_TYPE="Newly built native binary"
    fi
fi

if [ -n "$SELECTED_EXEC" ] && [ -f "$SELECTED_EXEC" ]; then
    echo "[Launcher] Launching: $SELECTED_EXEC [$EXEC_TYPE]"
    
    # If launching AppImage, ensure execution compatibility with or without FUSE
    if [[ "$SELECTED_EXEC" == *".AppImage" ]]; then
        export APPIMAGE_EXTRACT_AND_RUN=1
    fi
    
    exec "$SELECTED_EXEC" "${PASSTHROUGH_ARGS[@]}"
else
    echo "[Error] Could not find or build a valid executable to launch."
    echo "Tip: Run './setup/build_image.sh' to compile, or './setup/launch-dev.sh' to run live from source."
    exit 1
fi
