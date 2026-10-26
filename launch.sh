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

is_outdated() {
    local target="$1"
    if [ ! -f "$target" ]; then
        return 0
    fi
    local newer_files
    newer_files=$(find "$ROOT_DIR/src" "$ROOT_DIR/src-tauri/src" "$ROOT_DIR/index.html" "$ROOT_DIR/setup" "$ROOT_DIR/src-tauri/Cargo.toml" -type f -newer "$target" 2>/dev/null | head -n 1)
    if [ -n "$newer_files" ]; then
        return 0
    fi
    return 1
}

build_binary() {
    print_header
    if ! command -v cargo >/dev/null 2>&1; then
        echo ""
        echo "[Error] 'cargo' (Rust) was not found in PATH."
        echo "To compile Animated Linux, install Rust & Cargo first:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        exit 1
    fi
    echo "[Launcher] Compiling standalone native executable..."
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
    pnpm tauri build --no-bundle
    mkdir -p "$ROOT_DIR/dist"
    cp -f "$ROOT_DIR/src-tauri/target/release/animated-linux" "$ROOT_DIR/dist/animated-linux-x86_64" 2>/dev/null || true
    echo "[Launcher] Build finished."
}

install_autostart_entry() {
    local autostart_dir="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
    local autostart_file="$autostart_dir/animated-linux.desktop"

    echo "[Launcher] Installing system startup autostart entry..."
    mkdir -p "$autostart_dir"

    cat << EOF > "$autostart_file"
[Desktop Entry]
Type=Application
Name=Animated Linux
GenericName=Animated Wallpaper Engine
Comment=Hardware-accelerated animated wallpaper engine for Linux
Exec="$ROOT_DIR/launch.sh" --autostart --minimized
Icon=animated-linux
Terminal=false
Categories=Utility;Settings;
X-GNOME-Autostart-enabled=true
EOF

    chmod +x "$autostart_file"
    echo "[OK] Autostart entry installed: $autostart_file"
    echo "     Exec -> $ROOT_DIR/launch.sh --autostart --minimized"
}

uninstall_autostart_entry() {
    local autostart_file="${XDG_CONFIG_HOME:-$HOME/.config}/autostart/animated-linux.desktop"
    if [ -f "$autostart_file" ]; then
        rm -f "$autostart_file"
        echo "[Launcher] Removed autostart entry: $autostart_file"
    else
        echo "[Launcher] Autostart entry not found: $autostart_file"
    fi
}

show_help() {
    print_header
    echo "Usage: ./launch.sh [OPTIONS] [-- APP_ARGS...]"
    echo ""
    echo "Launches Animated Linux through this master launch script."
    echo ""
    echo "Options:"
    echo "  --dev, --source       Run directly from source with live reload (HMR)"
    echo "  --build               Compile or recompile the standalone native executable"
    echo "  --background, -b      Run silently in the background (no GUI window)"
    echo "  --install-desktop     Create/update the desktop start menu entry pointing to this launcher"
    echo "  --uninstall-desktop   Remove the desktop start menu entry"
    echo "  --install-autostart   Create/update the background startup entry pointing to this launcher"
    echo "  --uninstall-autostart Remove the startup autostart entry"
    echo "  -h, --help            Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./launch.sh                    # Launch (auto-recompiles if code changed)"
    echo "  ./launch.sh --background       # Launch silently in background / system tray"
    echo "  ./launch.sh --dev              # Run live directly from source code"
    echo "  ./launch.sh --install-desktop  # Register in desktop applications start menu"
    echo "  ./launch.sh --install-autostart# Register background startup on login"
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
        --background|-b)
            PASSTHROUGH_ARGS+=("--background")
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
        --install-autostart)
            install_autostart_entry
            exit 0
            ;;
        --uninstall-autostart)
            uninstall_autostart_entry
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

# If explicit build requested, compile the executable only
if [ "$ACTION" = "build" ]; then
    build_binary
    echo "[OK] Standalone native executable built: $ROOT_DIR/dist/animated-linux-x86_64"
    exit 0
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

if [ -z "$FOUND_BINARY" ] || is_outdated "$FOUND_BINARY"; then
    if [ -n "$FOUND_BINARY" ]; then
        echo "[Launcher] Source changes detected since previous build."
    fi
    echo "[Launcher] Auto-building up-to-date standalone release binary..."
    build_binary
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
