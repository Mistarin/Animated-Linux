use super::backend::{PlaybackSettings, WallpaperBackend, WallpaperItem};
use crate::new_host_command;
use std::path::Path;

pub struct KdePlasmaBackend;

impl KdePlasmaBackend {
    pub fn new() -> Self {
        Self::ensure_local_patch();
        Self
    }

    pub fn ensure_local_patch() {
        let local_dir = match dirs::data_local_dir() {
            Some(d) => d.join("plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn"),
            None => return,
        };
        let sys_dir = Path::new("/usr/share/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn");

        if !sys_dir.exists() && !local_dir.exists() {
            return;
        }

        if !local_dir.exists() && sys_dir.exists() {
            if let Err(e) = Self::copy_dir_recursive(sys_dir, &local_dir) {
                eprintln!("[AnimatedLinux] Failed to copy Reborn plugin to user directory: {}", e);
                return;
            }
        }

        let main_xml = local_dir.join("contents/config/main.xml");
        let main_qml = local_dir.join("contents/ui/main.qml");

        let mut changed = false;

        if main_xml.exists() {
            if let Ok(mut xml) = std::fs::read_to_string(&main_xml) {
                let mut xml_changed = false;
                if !xml.contains("InactiveBlackScreen") {
                    xml = xml.replace(
                        "</group>",
                        "    <entry name=\"InactiveBlackScreen\" type=\"Bool\">\n      <label>Fade to black when inactive or paused</label>\n      <default>false</default>\n    </entry>\n  </group>",
                    );
                    xml_changed = true;
                }
                if !xml.contains("RestartOnResume") {
                    xml = xml.replace(
                        "</group>",
                        "    <entry name=\"RestartOnResume\" type=\"Bool\">\n      <label>Restart video from beginning on resume</label>\n      <default>false</default>\n    </entry>\n  </group>",
                    );
                    xml_changed = true;
                }
                if xml_changed {
                    let _ = std::fs::write(&main_xml, xml);
                    changed = true;
                }
            }
        }

        if main_qml.exists() {
            if let Ok(mut qml) = std::fs::read_to_string(&main_qml) {
                let mut qml_changed = false;
                if !qml.contains("inactiveBlackScreen") || qml.contains("InactiveBlackScreen || false") {
                    qml = qml.replace(
                        "property bool debugEnabled: main.configuration.DebugEnabled\n    property bool inactiveBlackScreen: main.configuration.InactiveBlackScreen || false\n    property bool restartOnResume: main.configuration.RestartOnResume || false",
                        "property bool debugEnabled: main.configuration.DebugEnabled === true || main.configuration.DebugEnabled === \"true\"\n    property bool inactiveBlackScreen: main.configuration.InactiveBlackScreen === true || main.configuration.InactiveBlackScreen === \"true\"\n    property bool restartOnResume: main.configuration.RestartOnResume === true || main.configuration.RestartOnResume === \"true\"",
                    );
                    qml = qml.replace(
                        "property bool debugEnabled: main.configuration.DebugEnabled",
                        "property bool debugEnabled: main.configuration.DebugEnabled === true || main.configuration.DebugEnabled === \"true\"\n    property bool inactiveBlackScreen: main.configuration.InactiveBlackScreen === true || main.configuration.InactiveBlackScreen === \"true\"\n    property bool restartOnResume: main.configuration.RestartOnResume === true || main.configuration.RestartOnResume === \"true\"",
                    );
                    qml_changed = true;
                }
                if !qml.contains("blackOverlay") {
                    let overlay_qml = "            resumeLastVideo: main.configuration.ResumeLastVideo\n        }\n\n        Rectangle {\n            id: blackOverlay\n            anchors.fill: parent\n            color: \"#000000\"\n            opacity: (main.inactiveBlackScreen && !main.playing && main.videosConfig.length > 0) ? 1.0 : 0.0\n            visible: main.inactiveBlackScreen && (!main.playing || opacity > 0.001)\n            z: 99\n            Behavior on opacity {\n                NumberAnimation {\n                    duration: 350\n                    easing.type: Easing.InOutQuad\n                }\n            }\n        }\n\n        Connections {\n            target: main\n            ignoreUnknownSignals: true\n            function onRestartOnResumeChanged() {\n                if (!main.playing && main.restartOnResume) {\n                    try { player.player.position = 0; } catch(e) {}\n                }\n            }\n        }";
                    qml = qml.replace(
                        "            resumeLastVideo: main.configuration.ResumeLastVideo\n        }",
                        overlay_qml,
                    );
                    qml_changed = true;
                }
                if !qml.contains("player.player.position = 0") {
                    let pause_patch = "        onTriggered: {\n            player.pause();\n            if (main.restartOnResume) {\n                try {\n                    player.player.position = 0;\n                } catch(e) {}\n            }\n        }";
                    qml = qml.replace(
                        "        onTriggered: {\n            player.pause();\n        }",
                        pause_patch,
                    );
                    let play_patch = "        onTriggered: {\n            if (main.restartOnResume) {\n                try {\n                    player.player.position = 0;\n                } catch(e) {}\n            }\n            player.play();\n        }";
                    qml = qml.replace(
                        "        onTriggered: {\n            player.play();\n        }",
                        play_patch,
                    );
                    qml_changed = true;
                }
                if qml_changed {
                    let _ = std::fs::write(&main_qml, qml);
                    changed = true;
                }
            }
        }

        if changed {
            let _ = new_host_command("kbuildsycoca6").arg("--noincremental").output();
        }
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    fn run_plasma_script(script: &str) -> Result<String, String> {
        let output = new_host_command("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .arg("org.kde.PlasmaShell.evaluateScript")
            .arg(script)
            .output()
            .map_err(|e| format!("Failed to execute qdbus: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Plasma D-Bus script error: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn set_debug_enabled(&self, enabled: bool) -> Result<(), String> {
        let script = format!(
            r#"
var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
    d.writeConfig("DebugEnabled", {enabled});
    d.reloadConfig();
}}
"#,
            enabled = enabled
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }
}

impl WallpaperBackend for KdePlasmaBackend {
    fn name(&self) -> &'static str {
        "KDE Plasma 6 Native Backend"
    }

    fn is_available(&self) -> bool {
        // 1. Check if qdbus org.kde.plasmashell responds
        let plasma_ok = new_host_command("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !plasma_ok {
            return false;
        }

        // 2. Check if the Smart Video Wallpaper Reborn plugin is installed
        let local_path = dirs::data_local_dir()
            .map(|p| p.join("plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn").exists())
            .unwrap_or(false);
        let system_path = Path::new("/usr/share/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn").exists();

        local_path || system_path
    }

    fn apply(&self, wallpaper: &WallpaperItem, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let video_url = if wallpaper.file_path.starts_with("file://") {
            wallpaper.file_path.clone()
        } else {
            format!("file://{}", wallpaper.file_path)
        };

        let target_screen = screen.unwrap_or(-1);
        let fill_mode = settings.fill_mode;
        let pause_mode = match settings.pause_policy {
            0 => 3, // 3: Never pause in reborn
            1 => 3, // 1: Fullscreen Game/App: Reborn combines maximized and fullscreen in mode 0, so using 3 prevents freezing on normal maximized windows
            2 => 0, // 2: Maximized Window Active: Pauses on maximized or fullscreen windows
            3 => 1, // 3: Any Window Active: When a visible window is active
            _ => 3,
        };
        let check_active_screen = settings.pause_detection_mode == "individual";
        let debug_enabled = settings.debug_enabled;
        let mute_mode = if settings.is_muted { 5 } else { 4 }; // 5: Always mute, 4: Never mute
        let volume_val = (settings.volume as f64) / 100.0;
        let speed_val = settings.speed;
        let inactive_black = settings.inactive_black_screen && !settings.inactive_blur;
        let inactive_blur = settings.inactive_blur && !settings.inactive_black_screen;
        let restart_on_resume = settings.restart_on_resume;
        let blur_mode = if inactive_blur { 3 } else { 5 };
        let blur_radius = if inactive_blur { settings.blur_strength } else { 0 };

        // Plasma 6 Script to update wallpaper plugin on specified screen(s)
        // Smart Video Wallpaper Reborn requires VideoUrls to be a JSON array of video objects with "enabled": true
        let video_config_json = serde_json::json!([{
            "filename": video_url,
            "enabled": true,
            "duration": 0,
            "customDuration": 0,
            "playbackRate": speed_val,
            "loop": true
        }]).to_string();

        let video_url_json = serde_json::to_string(&video_url)
            .unwrap_or_else(|_| format!("\"{}\"", video_url));

        // Pre-kickstart PauseMode transition to ensure onPlayingChanged fires and triggers main.play()
        let kickstart_script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.wallpaperPlugin = "luisbocanegra.smart.video.wallpaper.reborn";
        d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
        d.writeConfig("PauseMode", 99);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen
        );
        let _ = Self::run_plasma_script(&kickstart_script);
        std::thread::sleep(std::time::Duration::from_millis(30));

        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
var videoUrl = {video_url_json};
var videoConfig = {video_config_json};

for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.wallpaperPlugin = "luisbocanegra.smart.video.wallpaper.reborn";
        d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
        d.writeConfig("VideoUrls", JSON.stringify(videoConfig));
        d.writeConfig("LastVideo", videoUrl);
        d.writeConfig("ResumeLastVideo", false);
        d.writeConfig("LastVideoPosition", 0);
        d.writeConfig("PlaybackRate", {speed_val});
        d.writeConfig("FillMode", {fill_mode});
        d.writeConfig("CheckWindowsActiveScreen", {check_active_screen});
        d.writeConfig("DebugEnabled", {debug_enabled});
        d.writeConfig("InactiveBlackScreen", {inactive_black});
        d.writeConfig("InactiveBlurScreen", {inactive_blur});
        d.writeConfig("RestartOnResume", {restart_on_resume});
        d.writeConfig("BlurMode", {blur_mode});
        d.writeConfig("BlurRadius", {blur_radius});
        d.writeConfig("PauseMode", {pause_mode});
        d.writeConfig("MuteMode", {mute_mode});
        d.writeConfig("Volume", {volume_val});
        d.reloadConfig();
    }}
}}
print("Applied successfully to target " + target);
"#,
            target_screen = target_screen,
            video_url_json = video_url_json,
            video_config_json = video_config_json,
            speed_val = speed_val,
            fill_mode = fill_mode,
            check_active_screen = check_active_screen,
            debug_enabled = debug_enabled,
            inactive_black = inactive_black,
            inactive_blur = inactive_blur,
            restart_on_resume = restart_on_resume,
            blur_mode = blur_mode,
            blur_radius = blur_radius,
            pause_mode = pause_mode,
            mute_mode = mute_mode,
            volume_val = volume_val
        );

        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn pause(&self, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let inactive_black = settings.inactive_black_screen && !settings.inactive_blur;
        let inactive_blur = settings.inactive_blur && !settings.inactive_black_screen;
        let restart_on_resume = settings.restart_on_resume;
        let blur_mode = if inactive_blur { 3 } else { 5 };
        let blur_radius = if inactive_blur { settings.blur_strength } else { 0 };
        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
        d.writeConfig("InactiveBlackScreen", {inactive_black});
        d.writeConfig("InactiveBlurScreen", {inactive_blur});
        d.writeConfig("RestartOnResume", {restart_on_resume});
        d.writeConfig("BlurMode", {blur_mode});
        d.writeConfig("BlurRadius", {blur_radius});
        d.writeConfig("PauseMode", 99);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen,
            inactive_black = inactive_black,
            inactive_blur = inactive_blur,
            restart_on_resume = restart_on_resume,
            blur_mode = blur_mode,
            blur_radius = blur_radius
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn resume(&self, screen: Option<i32>, settings: &PlaybackSettings, _wallpaper: Option<&WallpaperItem>) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let pause_mode = match settings.pause_policy {
            0 => 3,
            1 => 3,
            2 => 0,
            3 => 1,
            _ => 3,
        };
        let check_active_screen = settings.pause_detection_mode == "individual";
        let debug_enabled = settings.debug_enabled;
        let inactive_black = settings.inactive_black_screen && !settings.inactive_blur;
        let inactive_blur = settings.inactive_blur && !settings.inactive_black_screen;
        let restart_on_resume = settings.restart_on_resume;
        let blur_mode = if inactive_blur { 3 } else { 5 };
        let blur_radius = if inactive_blur { settings.blur_strength } else { 0 };

        let restart_script = if settings.restart_on_resume {
            r#"
        d.writeConfig("ResumeLastVideo", false);
        d.writeConfig("LastVideoPosition", 0);
"#
        } else {
            ""
        };

        // Pre-kickstart PauseMode to ensure onPlayingChanged fires and triggers main.play()
        let kickstart_script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
        d.writeConfig("PauseMode", 99);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen
        );
        let _ = Self::run_plasma_script(&kickstart_script);
        std::thread::sleep(std::time::Duration::from_millis(30));

        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
{restart_script}        d.writeConfig("CheckWindowsActiveScreen", {check_active_screen});
        d.writeConfig("DebugEnabled", {debug_enabled});
        d.writeConfig("InactiveBlackScreen", {inactive_black});
        d.writeConfig("InactiveBlurScreen", {inactive_blur});
        d.writeConfig("RestartOnResume", {restart_on_resume});
        d.writeConfig("BlurMode", {blur_mode});
        d.writeConfig("BlurRadius", {blur_radius});
        d.writeConfig("PauseMode", {pause_mode});
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen,
            restart_script = restart_script,
            check_active_screen = check_active_screen,
            debug_enabled = debug_enabled,
            inactive_black = inactive_black,
            inactive_blur = inactive_blur,
            restart_on_resume = restart_on_resume,
            blur_mode = blur_mode,
            blur_radius = blur_radius,
            pause_mode = pause_mode
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn set_volume(&self, volume: u32, muted: bool) -> Result<(), String> {
        let mute_mode = if muted { 5 } else { 4 };
        let volume_val = (volume as f64) / 100.0;
        let script = format!(
            r#"
var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
    d.writeConfig("Volume", {volume_val});
    d.writeConfig("MuteMode", {mute_mode});
    d.reloadConfig();
}}
"#,
            volume_val = volume_val,
            mute_mode = mute_mode
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn set_speed(&self, speed: f32) -> Result<(), String> {
        let script = format!(
            r#"
var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
    var current = d.readConfig("VideoUrls");
    try {{
        var parsed = JSON.parse(current);
        for (var j = 0; j < parsed.length; j++) {{
            parsed[j].playbackRate = {speed};
        }}
        d.writeConfig("VideoUrls", JSON.stringify(parsed));
        d.writeConfig("PlaybackRate", {speed});
        d.reloadConfig();
    }} catch(e) {{}}
}}
"#,
            speed = speed
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn set_geometry(&self, fill_mode: u32, _pan_x: f32, _pan_y: f32) -> Result<(), String> {
        let script = format!(
            r#"
var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
    d.writeConfig("FillMode", {fill_mode});
    d.reloadConfig();
}}
"#,
            fill_mode = fill_mode
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn stop(&self, screen: Option<i32>, _settings: &PlaybackSettings) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let fallback_img = crate::wallpaper::WallpaperManager::find_default_system_wallpaper();
        let fallback_json = serde_json::to_string(&fallback_img).unwrap_or_else(|_| format!("\"{}\"", fallback_img));
        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.wallpaperPlugin = "org.kde.image";
        d.currentConfigGroup = Array("Wallpaper", "org.kde.image", "General");
        var existingImg = d.readConfig("Image");
        if (!existingImg || existingImg.length === 0) {{
            d.writeConfig("Image", {fallback_json});
        }}
        d.writeConfig("FillMode", 2);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen,
            fallback_json = fallback_json
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }
}
