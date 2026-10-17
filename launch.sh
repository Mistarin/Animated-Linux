#!/usr/bin/env bash
# Copyright (C) 2026 Animated Linux Contributors
# Licensed under GNU GPL-3.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

APP_NAME="animated-linux"
ICON_PATH="$ROOT_DIR/src-tauri/icons/128x128.png"

# Force GTK and WebKitGTK to prefer dark theme
export GTK_THEME="Adwaita:dark"
export GTK_APPLICATION_PREFER_DARK_THEME="1"

# Prevent WebKitGTK explicit sync crashes on Wayland (Error 71)
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"

print_header() {
    echo "================================================================"
    echo "          Animated Linux Launcher                               "
    echo "================================================================"
}

install_desktop_entry() {
    local apps_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
    local icons_dir="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/128x128/apps"
    local desktop_file="$apps_dir/animated-linux.desktop"

    echo "[Launcher] Installing desktop application entry..."
    mkdir -p "$apps_dir"
    mkdir -p "$icons_dir"

    # Install icon to user hicolor directory
    if [ -f "$ICON_PATH" ]; then
        cp -f "$ICON_PATH" "$icons_dir/animated-linux.png"
    fi

    cat << EOF > "$desktop_file"
[Desktop Entry]
Type=Application
Name=Animated Linux
GenericName=Animated Wallpaper Engine
Comment=Hardware-accelerated animated wallpaper engine for Linux
Exec="$ROOT_DIR/launch.sh"
Icon=animated-linux
Terminal=false
Categories=Utility;Graphics;Settings;
Keywords=wallpaper;video;animated;engine;plasma;kde;
StartupWMClass=animated-linux
EOF

    chmod +x "$desktop_file"

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$apps_dir" 2>/dev/null || true
    fi
    if command -v kbuildsycoca6 >/dev/null 2>&1; then
        kbuildsycoca6 >/dev/null 2>&1 || true
    fi

    echo "[OK] Desktop entry installed: $desktop_file"
    echo "     Exec -> $ROOT_DIR/launch.sh"
}

uninstall_desktop_entry() {
    local apps_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
    local desktop_file="$apps_dir/animated-linux.desktop"

    if [ -f "$desktop_file" ]; then
        rm -f "$desktop_file"
        echo "[Launcher] Removed $desktop_file"
    else
        echo "[Launcher] Desktop entry not found: $desktop_file"
    fi

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$apps_dir" 2>/dev/null || true
    fi
    if command -v kbuildsycoca6 >/dev/null 2>&1; then
        kbuildsycoca6 >/dev/null 2>&1 || true
    fi
}

show_help() {
    print_header
    echo "Usage: ./launch.sh [OPTIONS] [-- APP_ARGS...]"
    echo ""
    echo "Launches Animated Linux executable through this launch script."
    echo ""
    echo "Options:"
    echo "  --dev, --source     Run directly from source with live reload (HMR)"
    echo "  --build             Compile the standalone native executable"
    echo "  --install-desktop   Create/update the desktop menu entry pointing to this launcher"
    echo "  --uninstall-desktop Remove the desktop menu entry"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./launch.sh                  # Launch native executable with proper environment"
    echo "  ./launch.sh --dev            # Run live directly from source code"
    echo "  ./launch.sh --build          # Build or recompile the standalone executable"
    echo "  ./launch.sh --install-desktop# Register Animated Linux in desktop applications menu"
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
ACTION="run" # run | dev | build
PASSTHROUGH_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dev|--source)
            ACTION="dev"
            shift
            ;;
        --build|--compile)
            ACTION="build"
            shift
            ;;
        --install-desktop|-i)
            install_desktop_entry
            exit 0
            ;;
        --uninstall-desktop)
            uninstall_desktop_entry
            exit 0
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
if [ "$ACTION" = "dev" ]; then
    exec "$ROOT_DIR/setup/launch-dev.sh" "${PASSTHROUGH_ARGS[@]}"
fi

# If explicit build requested, compile the single executable only
if [ "$ACTION" = "build" ]; then
    print_header
    echo "[Launcher] Compiling standalone native executable..."
    bash "$ROOT_DIR/setup/build_image.sh" --binary-only
    echo "[Launcher] Build finished. Launching executable..."
fi

# Locate existing native executable
FOUND_BINARY=""
if [ -f "$ROOT_DIR/src-tauri/target/release/animated-linux" ] && [ -x "$ROOT_DIR/src-tauri/target/release/animated-linux" ]; then
    FOUND_BINARY="$ROOT_DIR/src-tauri/target/release/animated-linux"
elif [ -f "$ROOT_DIR/dist/animated-linux-x86_64" ] && [ -x "$ROOT_DIR/dist/animated-linux-x86_64" ]; then
    FOUND_BINARY="$ROOT_DIR/dist/animated-linux-x86_64"
elif [ -f "$ROOT_DIR/src-tauri/target/debug/animated-linux" ] && [ -x "$ROOT_DIR/src-tauri/target/debug/animated-linux" ]; then
    FOUND_BINARY="$ROOT_DIR/src-tauri/target/debug/animated-linux"
fi

if [ -z "$FOUND_BINARY" ]; then
    print_header
    echo "[Launcher] No pre-compiled executable found in repository."
    if ! command -v cargo >/dev/null 2>&1; then
        echo ""
        echo "[Error] 'cargo' (Rust) was not found in PATH."
        echo "A fresh clone does not include binaries (they are gitignored)."
        echo "To run or compile Animated Linux, Rust & Cargo must be installed:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  source \$HOME/.cargo/env"
        echo ""
        exit 1
    fi
    echo "[Launcher] Building standalone native executable..."
    bash "$ROOT_DIR/setup/build_image.sh" --binary-only
    if [ -f "$ROOT_DIR/src-tauri/target/release/animated-linux" ]; then
        FOUND_BINARY="$ROOT_DIR/src-tauri/target/release/animated-linux"
    elif [ -f "$ROOT_DIR/dist/animated-linux-x86_64" ]; then
        FOUND_BINARY="$ROOT_DIR/dist/animated-linux-x86_64"
    fi
fi

if [ -n "$FOUND_BINARY" ] && [ -x "$FOUND_BINARY" ]; then
    exec "$FOUND_BINARY" "${PASSTHROUGH_ARGS[@]}"
else
    echo "[Error] Could not find or build executable: $FOUND_BINARY"
    echo "Tip: Run './launch.sh --dev' to run directly from source with live HMR."
    exit 1
fi
