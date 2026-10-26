// GPL-3.0 License
use super::backend::{PlaybackSettings, WallpaperBackend, WallpaperItem};
use crate::new_host_command;
use std::path::Path;

pub struct KdePlasmaMpvBackend;

impl KdePlasmaMpvBackend {
    pub fn new() -> Self {
        Self
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
}

impl WallpaperBackend for KdePlasmaMpvBackend {
    fn name(&self) -> &'static str {
        "KDE Plasma 6 Native Backend (MPV)"
    }

    fn is_available(&self) -> bool {
        let plasma_ok = new_host_command("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !plasma_ok {
            return false;
        }

        let local_path = dirs::data_local_dir()
            .map(|p| p.join("plasma/wallpapers/com.github.catsout.wallpaperEngineKde").exists())
            .unwrap_or(false);

        let system_path = Path::new("/usr/share/plasma/wallpapers/com.github.catsout.wallpaperEngineKde").exists();

        local_path || system_path
    }

    fn apply(&self, wallpaper: &WallpaperItem, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let video_path = if wallpaper.file_path.starts_with("file://") {
            wallpaper.file_path.clone()
        } else {
            format!("file://{}", wallpaper.file_path)
        };
        let video_source = format!("{}+video", video_path);
        let video_source_json = serde_json::to_string(&video_source)
            .unwrap_or_else(|_| format!("\"{}\"", video_source));

        let target_screen = screen.unwrap_or(-1);
        let display_mode = match settings.fill_mode {
            0 => 2, // Scale / Stretch
            1 => 0, // Aspect / Keep aspect
            2 => 1, // Crop / Zoom fill
            _ => 1,
        };
        let pause_mode = match settings.pause_policy {
            0 => 0, // Never pause
            1 => 5, // FullScreen
            2 => 2, // Max
            3 => 1, // Any (Pause when any window is open / not minimized)
            4 => 0, // Battery
            _ => 0,
        };
        let pause_on_bat = settings.pause_policy == 4;
        let pause_detection_mode = &settings.pause_detection_mode;
        let main_screen = settings.main_display_id;
        let is_muted = settings.is_muted;
        let volume = settings.volume;
        let speed = settings.speed;
        let inactive_black = settings.inactive_black_screen;
        let restart_on_resume = settings.restart_on_resume;
        let is_paused = settings.is_paused;

        let target_fps = if settings.target_fps > 0 { settings.target_fps } else { 30 };

        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
var mainScreen = {main_screen};
var detectionMode = "{pause_detection_mode}";
var videoSource = {video_source_json};

for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.wallpaperPlugin = "com.github.catsout.wallpaperEngineKde";
        d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
        d.writeConfig("VideoBackend", 1);
        d.writeConfig("WallpaperSource", videoSource);
        d.writeConfig("DisplayMode", {display_mode});
        d.writeConfig("PauseMode", {pause_mode});
        var filterByScreen = (detectionMode === "individual");
        if (detectionMode === "main") {{
            filterByScreen = (d.screen === mainScreen);
        }}
        d.writeConfig("PauseFilterByScreen", filterByScreen);
        d.writeConfig("PauseDetectionMode", detectionMode);
        d.writeConfig("MainDisplayId", mainScreen);
        d.writeConfig("PauseOnBatPower", {pause_on_bat});
        d.writeConfig("Fps", {target_fps});
        d.writeConfig("MuteAudio", {is_muted});
        d.writeConfig("Volume", {volume});
        d.writeConfig("Speed", {speed});
        d.writeConfig("InactiveBlackScreen", {inactive_black});
        d.writeConfig("MpvHwdec", "auto-safe");
        d.writeConfig("RestartOnResume", {restart_on_resume});
        d.writeConfig("UserPaused", {is_paused});
        d.reloadConfig();
    }}
}}
print("Applied MPV wallpaper to target " + target);
"#,
            target_screen = target_screen,
            main_screen = main_screen,
            pause_detection_mode = pause_detection_mode,
            video_source_json = video_source_json,
            display_mode = display_mode,
            pause_mode = pause_mode,
            pause_on_bat = pause_on_bat,
            target_fps = target_fps,
            is_muted = is_muted,
            volume = volume,
            speed = speed,
            inactive_black = inactive_black,
            restart_on_resume = restart_on_resume,
            is_paused = is_paused,
        );

        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn pause(&self, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let inactive_black = settings.inactive_black_screen;
        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
        d.writeConfig("InactiveBlackScreen", {inactive_black});
        d.writeConfig("UserPaused", true);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen,
            inactive_black = inactive_black
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn resume(&self, screen: Option<i32>, settings: &PlaybackSettings, wallpaper: Option<&WallpaperItem>) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let pause_mode = match settings.pause_policy {
            0 => 0,
            1 => 5,
            2 => 2,
            3 => 1,
            4 => 0,
            _ => 0,
        };
        let pause_on_bat = settings.pause_policy == 4;
        let pause_detection_mode = &settings.pause_detection_mode;
        let main_screen = settings.main_display_id;
        let inactive_black = settings.inactive_black_screen;
        let restart_on_resume = settings.restart_on_resume;
        let speed = settings.speed;
        let video_source_js = if let Some(wp) = wallpaper {
            let video_path = if wp.file_path.starts_with("file://") {
                wp.file_path.clone()
            } else {
                format!("file://{}", wp.file_path)
            };
            let source = format!("{}+video", video_path);
            serde_json::to_string(&source).unwrap_or_else(|_| format!("\"{}\"", source))
        } else {
            "\"\"".to_string()
        };

        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
var mainScreen = {main_screen};
var detectionMode = "{pause_detection_mode}";
var videoSource = {video_source_js};

for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        if (d.wallpaperPlugin !== "com.github.catsout.wallpaperEngineKde") {{
            d.wallpaperPlugin = "com.github.catsout.wallpaperEngineKde";
        }}
        d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
        if (videoSource.length > 0) {{
            d.writeConfig("WallpaperSource", videoSource);
        }}
        d.writeConfig("InactiveBlackScreen", {inactive_black});
        d.writeConfig("RestartOnResume", {restart_on_resume});
        d.writeConfig("PauseMode", {pause_mode});
        var filterByScreen = (detectionMode === "individual");
        if (detectionMode === "main") {{
            filterByScreen = (d.screen === mainScreen);
        }}
        d.writeConfig("PauseFilterByScreen", filterByScreen);
        d.writeConfig("PauseDetectionMode", detectionMode);
        d.writeConfig("MainDisplayId", mainScreen);
        d.writeConfig("PauseOnBatPower", {pause_on_bat});
        d.writeConfig("Speed", {speed});
        d.writeConfig("UserPaused", false);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen,
            main_screen = main_screen,
            pause_detection_mode = pause_detection_mode,
            video_source_js = video_source_js,
            pause_mode = pause_mode,
            pause_on_bat = pause_on_bat,
            speed = speed,
            inactive_black = inactive_black,
            restart_on_resume = restart_on_resume,
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn set_volume(&self, volume: u32, muted: bool) -> Result<(), String> {
        let script = format!(
            r#"
var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
    d.writeConfig("Volume", {volume});
    d.writeConfig("MuteAudio", {muted});
    d.reloadConfig();
}}
"#,
            volume = volume,
            muted = muted
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
    d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
    d.writeConfig("Speed", {speed});
    d.reloadConfig();
}}
"#,
            speed = speed
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn set_geometry(&self, fill_mode: u32, _pan_x: f32, _pan_y: f32) -> Result<(), String> {
        let display_mode = match fill_mode {
            0 => 2, // Scale / Stretch
            1 => 0, // Aspect / Keep aspect
            2 => 1, // Crop / Zoom fill
            _ => 1,
        };
        let script = format!(
            r#"
var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
    d.writeConfig("DisplayMode", {display_mode});
    d.reloadConfig();
}}
"#,
            display_mode = display_mode
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
