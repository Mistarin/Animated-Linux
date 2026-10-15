#!/usr/bin/env bash
# GPL-3.0 License - Animated Linux KDE Plasma MPV Plugin Installer
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

LOCAL_WP_DIR="${HOME}/.local/share/plasma/wallpapers/com.github.catsout.wallpaperEngineKde"
LOCAL_QML_DIR="${HOME}/.local/lib/qt6/qml/com/github/catsout/wallpaperEngineKde"
ARCHIVE_PATH="${ROOT_DIR}/bin/plasma6-wallpaper-engine.tar.zst"

# Check if system package is already present
if [[ -d "/usr/share/plasma/wallpapers/com.github.catsout.wallpaperEngineKde" && -f "/usr/lib/qt6/qml/com/github/catsout/wallpaperEngineKde/libWallpaperEngineKde.so" ]]; then
    echo "[Installer] System-wide plasma6-wallpapers-wallpaper-engine is already installed."
    exit 0
fi

# Ensure user directories exist
mkdir -p "${HOME}/.local/share/plasma/wallpapers"
mkdir -p "${HOME}/.local/lib/qt6/qml"
mkdir -p "${HOME}/.config/plasma-workspace/env"

# Extract from bundled archive if not already present in user directories
if [[ ! -d "${LOCAL_WP_DIR}" || ! -f "${LOCAL_QML_DIR}/libWallpaperEngineKde.so" ]]; then
    if [[ -f "${ARCHIVE_PATH}" ]]; then
        echo "[Installer] Extracting bundled KDE Plasma MPV plugin to user environment..."
        TMP_EXTRACT="$(mktemp -d)"
        tar -xf "${ARCHIVE_PATH}" -C "${TMP_EXTRACT}"
        
        cp -r "${TMP_EXTRACT}/usr/share/plasma/wallpapers/com.github.catsout.wallpaperEngineKde" "${HOME}/.local/share/plasma/wallpapers/"
        mkdir -p "${LOCAL_QML_DIR}"
        cp -r "${TMP_EXTRACT}/usr/lib/qt6/qml/com/github/catsout/wallpaperEngineKde/"* "${LOCAL_QML_DIR}/"
        rm -rf "${TMP_EXTRACT}"
        echo "[Installer] Plugin files installed successfully into ~/.local."
    else
        echo "[Installer] Warning: Bundled archive not found at ${ARCHIVE_PATH}."
    fi
fi

# Ensure QML2_IMPORT_PATH is configured for Plasma workspace
ENV_SCRIPT="${HOME}/.config/plasma-workspace/env/set_qml_path.sh"
if [[ ! -f "${ENV_SCRIPT}" ]] || ! grep -q "QML2_IMPORT_PATH" "${ENV_SCRIPT}"; then
    echo 'export QML2_IMPORT_PATH="$HOME/.local/lib/qt6/qml:${QML2_IMPORT_PATH:-}"' > "${ENV_SCRIPT}"
    chmod +x "${ENV_SCRIPT}"
    echo "[Installer] Registered ~/.local/lib/qt6/qml in plasma-workspace environment."
fi

# Update current user session environment via systemd if available
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user set-environment QML2_IMPORT_PATH="${HOME}/.local/lib/qt6/qml:${QML2_IMPORT_PATH:-}" || true
    # If plasmashell is running and hasn't picked up QML2_IMPORT_PATH, restart it so the plugin loads
    if pgrep -x plasmashell >/dev/null 2>&1; then
        PLASMA_PID="$(pgrep -x plasmashell | head -n1)"
        if ! grep -q "QML2_IMPORT_PATH" "/proc/${PLASMA_PID}/environ" 2>/dev/null; then
            echo "[Installer] Reloading plasma-plasmashell to register updated QML plugin paths..."
            systemctl --user restart plasma-plasmashell.service || true
        fi
    fi
fi

echo "[Installer] KDE Plasma MPV plugin setup is complete."

