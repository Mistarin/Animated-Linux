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
    if "InactiveBlurScreen" not in content:
        content = content.replace("</group>", """    <entry name="InactiveBlurScreen" type="Bool">
      <label>Fade to blur when inactive or paused</label>
      <default>false</default>
    </entry>
  </group>""")
        changed = True
    if "BlurRadius" not in content:
        content = content.replace("</group>", """    <entry name="BlurRadius" type="Int">
      <label>Blur strength radius</label>
      <default>32</default>
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
    target_props = """property int    pauseBatPercent: wallpaper.configuration.PauseBatPercent
    property bool   inactiveBlack: wallpaper.configuration.InactiveBlackScreen === true || wallpaper.configuration.InactiveBlackScreen === "true"
    property bool   inactiveBlur: wallpaper.configuration.InactiveBlurScreen === true || wallpaper.configuration.InactiveBlurScreen === "true"
    property int    blurRadius: wallpaper.configuration.BlurRadius ? parseInt(wallpaper.configuration.BlurRadius) : 32
    property bool   restartOnResume: wallpaper.configuration.RestartOnResume === true || wallpaper.configuration.RestartOnResume === "true"
    property bool   userPaused: wallpaper.configuration.UserPaused === true || wallpaper.configuration.UserPaused === "true" """

    if "inactiveBlur" not in content or "|| false" in content:
        import re
        content = re.sub(
            r'property int\s+pauseBatPercent:\s*wallpaper\.configuration\.PauseBatPercent[\s\S]*?(?=\s+property var|\s+// auto pause|\s+property string steamlibrary)',
            target_props,
            content
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

    if "Qt5Compat.GraphicalEffects" not in content:
        content = content.replace("import QtQuick 2.5", "import QtQuick 2.15\nimport Qt5Compat.GraphicalEffects")
        changed = True

    # Strip legacy imperative opacity overrides
    content = content.replace("        if (background.inactiveBlack) {\n            fadeRect.opacity = 0.0;\n        }\n", "")
    content = content.replace("        if (background.inactiveBlack) {\n            fadeRect.opacity = 1.0;\n        }\n", "")
    content = content.replace("        if (background.inactiveBlack && !background.ok) {\n            fadeRect.opacity = 1.0;\n        } else {\n            fadeRect.opacity = 0.0;\n        }\n", "")

    if 'seek", "0", "absolute"' not in content:
        content = content.replace(
            "function play(){\n        // stop pause time",
            "function play(){\n        // stop pause time\n        pauseTimer.stop();\n        if (background.restartOnResume) {\n            player.command([\"seek\", \"0\", \"absolute\"]);\n        }"
        )
        changed = True

    effects_block = """    FastBlur {
        id: blurEffect
        anchors.fill: player
        source: player
        radius: (background.inactiveBlur && !background.ok) ? background.blurRadius : 0
        visible: radius > 0.001
        z: 998
        Behavior on radius {
            NumberAnimation { duration: 350; easing.type: Easing.InOutQuad }
        }
    }

    Rectangle {
        id: fadeRect
        anchors.fill: parent
        color: "#000000"
        opacity: (background.inactiveBlack && !background.ok) ? 1.0 : 0.0
        visible: opacity > 0.001
        z: 999
        Behavior on opacity {
            NumberAnimation { duration: 350; easing.type: Easing.InOutQuad }
        }
    }

    Component.onCompleted:{"""

    if "blurEffect" not in content:
        import re
        if "Rectangle {\n        id: fadeRect" in content:
            content = re.sub(r'Rectangle\s*\{\s*id:\s*fadeRect[\s\S]*?Component\.onCompleted\s*:\s*\{', effects_block, content)
        elif "Component.onCompleted:{" in content:
            content = content.replace("Component.onCompleted:{", effects_block)
        changed = True

    with open(mpv_qml, "w", encoding="utf-8") as f:
        f.write(content)
    print("[Installer] Injected FastBlur and declarative fadeRect into Mpv.qml")

# 4. Patch QtMultimedia.qml
if os.path.isfile(qt_qml):
    with open(qt_qml, "r", encoding="utf-8") as f:
        content = f.read()
    changed = False

    if "Qt5Compat.GraphicalEffects" not in content:
        content = content.replace("import QtQuick 2.5", "import QtQuick 2.15\nimport Qt5Compat.GraphicalEffects")
        changed = True

    content = content.replace("        if (background.inactiveBlack) {\n            fadeRect.opacity = 0.0;\n        }\n", "")
    content = content.replace("        if (background.inactiveBlack) {\n            fadeRect.opacity = 1.0;\n        }\n", "")
    content = content.replace("        if (background.inactiveBlack && !background.ok) {\n            fadeRect.opacity = 1.0;\n        } else {\n            fadeRect.opacity = 0.0;\n        }\n", "")

    if "player.position = 0;" not in content:
        content = content.replace(
            "function play(){\n        pauseTimer.stop();",
            "function play(){\n        pauseTimer.stop();\n        if (background.restartOnResume) {\n            player.position = 0;\n        }"
        )
        changed = True

    qt_effects_block = """    FastBlur {
        id: blurEffect
        anchors.fill: videoView
        source: videoView
        radius: (background.inactiveBlur && !background.ok) ? background.blurRadius : 0
        visible: radius > 0.001
        z: 998
        Behavior on radius {
            NumberAnimation { duration: 350; easing.type: Easing.InOutQuad }
        }
    }

    Rectangle {
        id: fadeRect
        anchors.fill: parent
        color: "#000000"
        opacity: (background.inactiveBlack && !background.ok) ? 1.0 : 0.0
        visible: opacity > 0.001
        z: 999
        Behavior on opacity {
            NumberAnimation { duration: 350; easing.type: Easing.InOutQuad }
        }
    }

    Component.onCompleted:{"""

    if "blurEffect" not in content:
        import re
        if "Rectangle {\n        id: fadeRect" in content:
            content = re.sub(r'Rectangle\s*\{\s*id:\s*fadeRect[\s\S]*?Component\.onCompleted\s*:\s*\{', qt_effects_block, content)
        elif "Component.onCompleted:{" in content:
            content = content.replace("Component.onCompleted:{", qt_effects_block)
        changed = True

    with open(qt_qml, "w", encoding="utf-8") as f:
        f.write(content)
    print("[Installer] Injected FastBlur and declarative fadeRect into QtMultimedia.qml")

# 4. Patch Smart Video Wallpaper Reborn if present
reborn_sys = "/usr/share/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn"
reborn_user = os.path.expanduser("~/.local/share/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn")
if os.path.isdir(reborn_sys) and not os.path.isdir(reborn_user):
    import shutil
    shutil.copytree(reborn_sys, reborn_user)
    print("[Installer] Copied Reborn plugin to user directory")

if os.path.isdir(reborn_user):
    reborn_xml = os.path.join(reborn_user, "contents/config/main.xml")
    reborn_qml = os.path.join(reborn_user, "contents/ui/main.qml")
    if os.path.isfile(reborn_xml):
        with open(reborn_xml, "r", encoding="utf-8") as f:
            xml_c = f.read()
        x_changed = False
        if "InactiveBlackScreen" not in xml_c:
            xml_c = xml_c.replace("</group>", """    <entry name="InactiveBlackScreen" type="Bool">
      <label>Fade to black when inactive or paused</label>
      <default>false</default>
    </entry>
  </group>""")
            x_changed = True
        if "RestartOnResume" not in xml_c:
            xml_c = xml_c.replace("</group>", """    <entry name="RestartOnResume" type="Bool">
      <label>Restart video from beginning on resume</label>
      <default>false</default>
    </entry>
  </group>""")
            x_changed = True
        if x_changed:
            with open(reborn_xml, "w", encoding="utf-8") as f:
                f.write(xml_c)
            print("[Installer] Injected InactiveBlackScreen & RestartOnResume into Reborn main.xml")
    if os.path.isfile(reborn_qml):
        with open(reborn_qml, "r", encoding="utf-8") as f:
            qml_c = f.read()
        q_changed = False
        if "inactiveBlackScreen" not in qml_c or "|| false" in qml_c:
            qml_c = qml_c.replace(
                "property bool debugEnabled: main.configuration.DebugEnabled\n    property bool inactiveBlackScreen: main.configuration.InactiveBlackScreen || false\n    property bool restartOnResume: main.configuration.RestartOnResume || false",
                "property bool debugEnabled: main.configuration.DebugEnabled === true || main.configuration.DebugEnabled === \"true\"\\n    property bool inactiveBlackScreen: main.configuration.InactiveBlackScreen === true || main.configuration.InactiveBlackScreen === \"true\"\\n    property bool restartOnResume: main.configuration.RestartOnResume === true || main.configuration.RestartOnResume === \"true\""
            )
            qml_c = qml_c.replace(
                "property bool debugEnabled: main.configuration.DebugEnabled",
                "property bool debugEnabled: main.configuration.DebugEnabled === true || main.configuration.DebugEnabled === \"true\"\\n    property bool inactiveBlackScreen: main.configuration.InactiveBlackScreen === true || main.configuration.InactiveBlackScreen === \"true\"\\n    property bool restartOnResume: main.configuration.RestartOnResume === true || main.configuration.RestartOnResume === \"true\""
            )
            q_changed = True
        if "blackOverlay" not in qml_c:
            overlay_qml = """            resumeLastVideo: main.configuration.ResumeLastVideo
        }

        Rectangle {
            id: blackOverlay
            anchors.fill: parent
            color: "#000000"
            opacity: (main.inactiveBlackScreen && !main.playing && main.videosConfig.length > 0) ? 1.0 : 0.0
            visible: opacity > 0.001
            z: 99
            Behavior on opacity {
                NumberAnimation {
                    duration: 350
                    easing.type: Easing.InOutQuad
                }
            }
        }"""
            qml_c = qml_c.replace("            resumeLastVideo: main.configuration.ResumeLastVideo\\n        }", overlay_qml)
            q_changed = True
        if "player.player.position = 0" not in qml_c:
            qml_c = qml_c.replace(
                "player.pause();",
                "player.pause();\\n            if (main.restartOnResume) {\\n                try {\\n                    player.player.position = 0;\\n                } catch(e) {}\\n            }"
            )
            qml_c = qml_c.replace(
                "player.play();",
                "if (main.restartOnResume) {\\n                try {\\n                    player.player.position = 0;\\n                } catch(e) {}\\n            }\\n            player.play();"
            )
            q_changed = True
        if q_changed:
            with open(reborn_qml, "w", encoding="utf-8") as f:
                f.write(qml_c)
            print("[Installer] Injected blackOverlay and restartOnResume into Reborn main.qml")
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

