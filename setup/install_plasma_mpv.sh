#!/usr/bin/env bash
# GPL-3.0 License - Animated Linux KDE Plasma MPV Plugin Installer
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

LOCAL_WP_DIR="${HOME}/.local/share/plasma/wallpapers/com.github.catsout.wallpaperEngineKde"
LOCAL_QML_DIR="${HOME}/.local/lib/qt6/qml/com/github/catsout/wallpaperEngineKde"
ARCHIVE_PATH="${ROOT_DIR}/bin/plasma6-wallpaper-engine.tar.zst"

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
fi

# Automated QML Enhancements Patcher (Fade to Black, Restart on Resume, User Pause)
if [[ -d "${LOCAL_WP_DIR}" ]] && command -v python3 >/dev/null 2>&1; then
    python3 - << 'PYEOF'
import os

wp_dir = os.path.expanduser("~/.local/share/plasma/wallpapers/com.github.catsout.wallpaperEngineKde")
main_xml = os.path.join(wp_dir, "contents/config/main.xml")
main_qml = os.path.join(wp_dir, "contents/ui/main.qml")
mpv_qml = os.path.join(wp_dir, "contents/ui/backend/Mpv.qml")
qt_qml = os.path.join(wp_dir, "contents/ui/backend/QtMultimedia.qml")

# 1. Patch main.xml
if os.path.isfile(main_xml):
    with open(main_xml, "r", encoding="utf-8") as f:
        content = f.read()
    changed = False
    if "InactiveBlackScreen" not in content:
        content = content.replace("</group>", """    <entry name="InactiveBlackScreen" type="Bool">
      <label>Fade to black when inactive or paused</label>
      <default>false</default>
    </entry>
  </group>""")
        changed = True
    if "RestartOnResume" not in content:
        content = content.replace("</group>", """    <entry name="RestartOnResume" type="Bool">
      <label>Restart animation from beginning on resume</label>
      <default>false</default>
    </entry>
  </group>""")
        changed = True
    if "UserPaused" not in content:
        content = content.replace("</group>", """    <entry name="UserPaused" type="Bool">
      <label>User paused state</label>
      <default>false</default>
    </entry>
  </group>""")
        changed = True
    if changed:
        with open(main_xml, "w", encoding="utf-8") as f:
            f.write(content)
        print("[Installer] Injected config entries into main.xml")

# 2. Patch main.qml
if os.path.isfile(main_qml):
    with open(main_qml, "r", encoding="utf-8") as f:
        content = f.read()
    changed = False
    if "userPaused" not in content:
        content = content.replace(
            "property int    pauseBatPercent: wallpaper.configuration.PauseBatPercent",
            "property int    pauseBatPercent: wallpaper.configuration.PauseBatPercent\n    property bool   inactiveBlack: wallpaper.configuration.InactiveBlackScreen || false\n    property bool   restartOnResume: wallpaper.configuration.RestartOnResume || false\n    property bool   userPaused: wallpaper.configuration.UserPaused || false"
        )
        changed = True
    if "!userPaused" not in content:
        content = content.replace(
            "property bool   ok: !windowModel.reqPause && !powerSource.reqPause",
            "property bool   ok: !windowModel.reqPause && !powerSource.reqPause && !userPaused"
        )
        changed = True
    if changed:
        with open(main_qml, "w", encoding="utf-8") as f:
            f.write(content)
        print("[Installer] Injected userPaused and ok properties into main.qml")

# 3. Patch Mpv.qml
if os.path.isfile(mpv_qml):
    with open(mpv_qml, "r", encoding="utf-8") as f:
        content = f.read()
    changed = False
    if "fadeRect" not in content:
        fade_block = """    Rectangle {
        id: fadeRect
        anchors.fill: parent
        color: "#000000"
        opacity: (background.inactiveBlack && !background.ok) ? 1.0 : 0.0
        z: 999
        Behavior on opacity {
            NumberAnimation { duration: 350; easing.type: Easing.InOutQuad }
        }
    }

    Component.onCompleted:{
        background.nowBackend = 'mpv';
        videoItem.displayModeChanged();
        if (background.inactiveBlack && !background.ok) {
            fadeRect.opacity = 1.0;
        } else {
            fadeRect.opacity = 0.0;
        }
    }"""
        content = content.replace("Component.onCompleted:{", fade_block)
        changed = True
    if 'seek", "0", "absolute"' not in content:
        content = content.replace(
            "function play(){\n        // stop pause time",
            "function play(){\n        // stop pause time\n        pauseTimer.stop();\n        if (background.restartOnResume) {\n            player.command([\"seek\", \"0\", \"absolute\"]);\n        }\n        if (background.inactiveBlack) {\n            fadeRect.opacity = 0.0;\n        }"
        )
        changed = True
    if "fadeRect.opacity = 1.0" not in content:
        content = content.replace(
            "function pause(){\n        volumeFade.stop();\n        pauseTimer.start();",
            "function pause(){\n        volumeFade.stop();\n        pauseTimer.start();\n        if (background.inactiveBlack) {\n            fadeRect.opacity = 1.0;\n        }"
        )
        changed = True
    if changed:
        with open(mpv_qml, "w", encoding="utf-8") as f:
            f.write(content)
        print("[Installer] Injected fadeRect and restartOnResume into Mpv.qml")

# 4. Patch QtMultimedia.qml
if os.path.isfile(qt_qml):
    with open(qt_qml, "r", encoding="utf-8") as f:
        content = f.read()
    changed = False
    if "fadeRect" not in content:
        fade_block = """    Rectangle {
        id: fadeRect
        anchors.fill: parent
        color: "#000000"
        opacity: (background.inactiveBlack && !background.ok) ? 1.0 : 0.0
        z: 999
        Behavior on opacity {
            NumberAnimation { duration: 350; easing.type: Easing.InOutQuad }
        }
    }

    Component.onCompleted:{
        background.nowBackend = "QtMultimedia";
        videoItem.displayModeChanged();
        if (background.inactiveBlack && !background.ok) {
            fadeRect.opacity = 1.0;
        } else {
            fadeRect.opacity = 0.0;
        }
    }"""
        content = content.replace("Component.onCompleted:{", fade_block)
        changed = True
    if "player.position = 0;" not in content:
        content = content.replace(
            "function play(){\n        pauseTimer.stop();",
            "function play(){\n        pauseTimer.stop();\n        if (background.restartOnResume) {\n            player.position = 0;\n        }\n        if (background.inactiveBlack) {\n            fadeRect.opacity = 0.0;\n        }"
        )
        changed = True
    if "fadeRect.opacity = 1.0" not in content:
        content = content.replace(
            "function pause(){\n        volumeFade.stop();\n        pauseTimer.start();",
            "function pause(){\n        volumeFade.stop();\n        pauseTimer.start();\n        if (background.inactiveBlack) {\n            fadeRect.opacity = 1.0;\n        }"
        )
        changed = True
    if changed:
        with open(qt_qml, "w", encoding="utf-8") as f:
            f.write(content)
        print("[Installer] Injected fadeRect and restartOnResume into QtMultimedia.qml")
PYEOF
fi

# Reload plasmashell if it has not yet registered the environment or if QML changed
if command -v systemctl >/dev/null 2>&1 && pgrep -x plasmashell >/dev/null 2>&1; then
    PLASMA_PID="$(pgrep -x plasmashell | head -n1)"
    if ! grep -q "QML2_IMPORT_PATH" "/proc/${PLASMA_PID}/environ" 2>/dev/null; then
        echo "[Installer] Reloading plasma-plasmashell to register updated QML plugin paths..."
        systemctl --user restart plasma-plasmashell.service || true
    fi
fi

echo "[Installer] KDE Plasma MPV plugin setup is complete."

