use super::backend::{PlaybackSettings, WallpaperBackend, WallpaperItem};
use std::process::Command;

pub struct KdePlasmaBackend;

impl KdePlasmaBackend {
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

impl WallpaperBackend for KdePlasmaBackend {
    fn name(&self) -> &'static str {
        "KDE Plasma 6 Native Backend"
    }

    fn is_available(&self) -> bool {
        // Check if qdbus org.kde.plasmashell responds
        if let Ok(output) = Command::new("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .output()
        {
            output.status.success()
        } else {
            false
        }
    }

    fn apply(&self, wallpaper: &WallpaperItem, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let video_url = if wallpaper.file_path.starts_with("file://") {
            wallpaper.file_path.clone()
        } else {
            format!("file://{}", wallpaper.file_path)
        };

        let target_screen = screen.unwrap_or(-1);
        let fill_mode = settings.fill_mode;
        // In Plasma 6, PauseMode 0 triggers windowModel errors and pauses on any maximized normal window.
        // PauseMode 3 (Never pause in QML) ensures continuous smooth video playback.
        // Performance pausing is managed directly by AetherPaper's detector.
        let pause_mode = 3;
        let mute_mode = if settings.is_muted { 5 } else { 4 }; // 5: Always mute, 4: Never mute
        let volume_val = (settings.volume as f64) / 100.0;
        let speed_val = settings.speed;

        // Plasma 6 Script to update wallpaper plugin on specified screen(s)
        // Notice: Smart Video Wallpaper Reborn requires VideoUrls to be a JSON array of video objects with "enabled": true
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
        d.writeConfig("ResumeLastVideo", true);
        d.writeConfig("PlaybackOverride", 0);
        d.writeConfig("PlaybackRate", {speed_val});
        d.writeConfig("FillMode", {fill_mode});
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
            pause_mode = pause_mode,
            mute_mode = mute_mode,
            volume_val = volume_val
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
        d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
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
        d.currentConfigGroup = Array("Wallpaper", "luisbocanegra.smart.video.wallpaper.reborn", "General");
        d.writeConfig("PauseMode", 3);
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
