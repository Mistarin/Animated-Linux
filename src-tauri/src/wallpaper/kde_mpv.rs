// GPL-3.0 License
use super::backend::{PlaybackSettings, WallpaperBackend, WallpaperItem};
use std::path::Path;
use std::process::Command;

pub struct KdePlasmaMpvBackend;

impl KdePlasmaMpvBackend {
    pub fn new() -> Self {
        Self
    }

    fn run_plasma_script(script: &str) -> Result<String, String> {
        let output = Command::new("qdbus")
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
        let plasma_ok = Command::new("qdbus")
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
            3 => 3, // Focus (Pause when any visible window is active)
            _ => 0,
        };
        let is_muted = settings.is_muted;
        let volume = settings.volume;
        let speed = settings.speed;

        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
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
        d.writeConfig("MuteAudio", {is_muted});
        d.writeConfig("Volume", {volume});
        d.writeConfig("Speed", {speed});
        d.reloadConfig();
    }}
}}
print("Applied MPV wallpaper to target " + target);
"#,
            target_screen = target_screen,
            video_source_json = video_source_json,
            display_mode = display_mode,
            pause_mode = pause_mode,
            is_muted = is_muted,
            volume = volume,
            speed = speed
        );

        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn pause(&self, screen: Option<i32>) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
        d.writeConfig("PauseMode", 1);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }

    fn resume(&self, screen: Option<i32>) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.currentConfigGroup = Array("Wallpaper", "com.github.catsout.wallpaperEngineKde", "General");
        d.writeConfig("PauseMode", 0);
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen
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

    fn stop(&self, screen: Option<i32>) -> Result<(), String> {
        let target_screen = screen.unwrap_or(-1);
        let script = format!(
            r#"
var allDesktops = desktops();
var target = {target_screen};
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    if (target === -1 || d.screen === target) {{
        d.wallpaperPlugin = "org.kde.image";
        d.currentConfigGroup = Array("Wallpaper", "org.kde.image", "General");
        d.reloadConfig();
    }}
}}
"#,
            target_screen = target_screen
        );
        Self::run_plasma_script(&script)?;
        Ok(())
    }
}
