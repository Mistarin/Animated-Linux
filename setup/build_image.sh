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
    
    # Ensure libsteam_api.so is located alongside binaries if present
    STEAM_SO="$(find "$ROOT_DIR/src-tauri/target" -name "libsteam_api.so" 2>/dev/null | head -n 1 || true)"
    if [ -n "$STEAM_SO" ]; then
        cp -u "$STEAM_SO" "$ROOT_DIR/src-tauri/target/release/" 2>/dev/null || true
        cp -u "$STEAM_SO" "$ROOT_DIR/dist/" 2>/dev/null || true
    fi
    echo "[OK] Binary built: dist/animated-linux-x86_64"
}

build_tauri_bundle() {
    echo "[Build] Packaging Standalone Desktop Bundle..."
    pnpm tauri build --no-bundle false || true
    
    # Locate generated bundle / AppImage if available
    BUNDLE_DIR="$ROOT_DIR/src-tauri/target/release/bundle"
    if [ -d "$BUNDLE_DIR" ]; then
        find "$BUNDLE_DIR" -type f \( -name "*.AppImage" -o -name "*.deb" \) -exec cp {} "$ROOT_DIR/dist/" \;
        echo "[OK] Bundles copied to dist/"
    fi
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
