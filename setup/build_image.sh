#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "================================================================"
echo "          Building Animated Linux Image & Distribution Packages  "
echo "================================================================"

MODE="all"
if [ $# -ge 1 ]; then
    case "$1" in
        --binary-only) MODE="binary" ;;
        --docker|--container) MODE="docker" ;;
        --bundle|--appimage) MODE="bundle" ;;
        *) MODE="all" ;;
    esac
fi

mkdir -p "$ROOT_DIR/dist"

build_native_binary() {
    echo "[1/2] Building Frontend & Release Binary with Embedded Assets..."
    cd "$ROOT_DIR"
    pnpm tauri build --no-bundle
    
    cp "$ROOT_DIR/src-tauri/target/release/animated-linux" "$ROOT_DIR/dist/animated-linux-x86_64"
    echo "[OK] Binary built: dist/animated-linux-x86_64"
}

build_tauri_bundle() {
    echo "[2/2] Packaging Standalone Desktop AppImage & Packages..."
    
    # 1. Build Debian package via Tauri bundler
    echo "Building Debian package bundle..."
    pnpm tauri build --bundles deb || true

    # 2. Package AppImage
    local appdir="$ROOT_DIR/src-tauri/target/release/bundle/appimage/Animated Linux.AppDir"
    local plugin_tool=""

    if [ -f "$HOME/.cache/tauri/linuxdeploy-plugin-appimage.AppImage" ]; then
        plugin_tool="$HOME/.cache/tauri/linuxdeploy-plugin-appimage.AppImage"
    elif command -v linuxdeploy-plugin-appimage >/dev/null 2>&1; then
        plugin_tool="$(command -v linuxdeploy-plugin-appimage)"
    elif command -v appimagetool >/dev/null 2>&1; then
        plugin_tool="$(command -v appimagetool)"
    fi

    if [ -z "$plugin_tool" ]; then
        echo "Fetching linuxdeploy-plugin-appimage for AppImage packaging..."
        mkdir -p "$HOME/.cache/tauri"
        curl -fsSL -o "$HOME/.cache/tauri/linuxdeploy-plugin-appimage.AppImage" "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage" || true
        if [ -f "$HOME/.cache/tauri/linuxdeploy-plugin-appimage.AppImage" ]; then
            chmod +x "$HOME/.cache/tauri/linuxdeploy-plugin-appimage.AppImage"
            plugin_tool="$HOME/.cache/tauri/linuxdeploy-plugin-appimage.AppImage"
        fi
    fi

    # Assemble AppDir payload
    mkdir -p "$appdir/usr/bin"
    mkdir -p "$appdir/usr/share/applications"
    mkdir -p "$appdir/usr/share/icons/hicolor/128x128/apps"
    cp -f "$ROOT_DIR/dist/animated-linux-x86_64" "$appdir/usr/bin/animated-linux"
    chmod +x "$appdir/usr/bin/animated-linux"

    cat << 'EOF' > "$appdir/usr/share/applications/Animated Linux.desktop"
[Desktop Entry]
Categories=Utility;Graphics;
Comment=Animated wallpaper engine for Linux
Exec=animated-linux
StartupWMClass=animated-linux
Icon=animated-linux
Name=Animated Linux
Terminal=false
Type=Application
EOF
    ln -sf "usr/share/applications/Animated Linux.desktop" "$appdir/Animated Linux.desktop"
    ln -sf "usr/share/applications/Animated Linux.desktop" "$appdir/animated-linux.desktop"

    if [ -f "$ROOT_DIR/src-tauri/icons/128x128.png" ]; then
        cp -f "$ROOT_DIR/src-tauri/icons/128x128.png" "$appdir/usr/share/icons/hicolor/128x128/apps/animated-linux.png"
        cp -f "$ROOT_DIR/src-tauri/icons/128x128.png" "$appdir/animated-linux.png"
        cp -f "$ROOT_DIR/src-tauri/icons/128x128.png" "$appdir/Animated Linux.png"
        ln -sf "Animated Linux.png" "$appdir/.DirIcon"
    fi

    if [ ! -f "$appdir/AppRun" ]; then
        if [ -f "$HOME/.cache/tauri/AppRun-x86_64" ]; then
            cp -f "$HOME/.cache/tauri/AppRun-x86_64" "$appdir/AppRun"
            chmod +x "$appdir/AppRun"
        else
            curl -fsSL -o "$appdir/AppRun" "https://github.com/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-x86_64" || true
            chmod +x "$appdir/AppRun" 2>/dev/null || true
        fi
    fi

    if [ -n "$plugin_tool" ] && [ -x "$plugin_tool" ]; then
        echo "Generating SquashFS AppImage package..."
        rm -f "$ROOT_DIR/Animated_Linux-x86_64.AppImage"
        rm -f "$ROOT_DIR/dist/Animated_Linux-x86_64.AppImage"
        (
            cd "$ROOT_DIR"
            APPIMAGE_EXTRACT_AND_RUN=1 "$plugin_tool" --appdir="$appdir" || true
        )

        local generated_appimage="$ROOT_DIR/Animated_Linux-x86_64.AppImage"
        if [ -f "$generated_appimage" ]; then
            mv -f "$generated_appimage" "$ROOT_DIR/dist/Animated_Linux-x86_64.AppImage"
            chmod +x "$ROOT_DIR/dist/Animated_Linux-x86_64.AppImage"
            echo "[OK] Standalone AppImage built: dist/Animated_Linux-x86_64.AppImage"

            # Deploy to standard ~/Applications folder if available
            local user_app_dir="$HOME/Applications"
            if [ -d "$user_app_dir" ]; then
                cp -f "$ROOT_DIR/dist/Animated_Linux-x86_64.AppImage" "$user_app_dir/Animated_Linux-x86_64.AppImage"
                chmod +x "$user_app_dir/Animated_Linux-x86_64.AppImage"
                echo "[OK] Deployed to $user_app_dir/Animated_Linux-x86_64.AppImage"
            fi
        fi
    fi

    # 3. Copy deb bundle to dist/
    local bundle_dir="$ROOT_DIR/src-tauri/target/release/bundle"
    if [ -d "$bundle_dir" ]; then
        find "$bundle_dir" -type f \( -name "*.AppImage" -o -name "*.deb" \) -exec cp -f {} "$ROOT_DIR/dist/" \; 2>/dev/null || true
    fi

    # Ensure native binary is present in dist/
    cp -f "$ROOT_DIR/src-tauri/target/release/animated-linux" "$ROOT_DIR/dist/animated-linux-x86_64"
}

build_container_image() {
    echo "[Build] Building Container Image..."
    CONTAINER_TOOL=""
    if command -v docker >/dev/null 2>&1; then
        CONTAINER_TOOL="docker"
    elif command -v podman >/dev/null 2>&1; then
        CONTAINER_TOOL="podman"
    fi

    if [ -n "$CONTAINER_TOOL" ]; then
        echo "Using $CONTAINER_TOOL to build animated-linux:latest..."
        "$CONTAINER_TOOL" build -t animated-linux:latest -f "$ROOT_DIR/setup/Dockerfile" "$ROOT_DIR"
        echo "[OK] Container image built successfully: animated-linux:latest"
    else
        echo "[Notice] Neither docker nor podman found. Skipping container image build."
    fi
}

case "$MODE" in
    binary)
        build_native_binary
        ;;
    bundle)
        build_native_binary
        build_tauri_bundle
        ;;
    docker)
        build_container_image
        ;;
    all)
        build_native_binary
        build_tauri_bundle
        if command -v docker >/dev/null 2>&1 || command -v podman >/dev/null 2>&1; then
            echo ""
            echo "Tip: Run './setup/build_image.sh --docker' to build the container image."
        fi
        ;;
esac

echo ""
echo "================================================================"
echo "[OK] Done! Outputs available in $ROOT_DIR/dist/"
echo "================================================================"
ls -lh "$ROOT_DIR/dist"
