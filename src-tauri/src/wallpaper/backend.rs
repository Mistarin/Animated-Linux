use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperItem {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub thumbnail_path: String,
    #[serde(default)]
    pub thumbnail_data: Option<String>,
    #[serde(default)]
    pub cached_qualities: Vec<u32>,
    pub duration_secs: f64,
    pub resolution: String,
    pub format: String,
    pub is_workshop: bool,
    pub workshop_id: Option<String>,
}

pub fn encode_thumbnail_base64(path: &Path) -> Option<String> {
    if let Ok(bytes) = std::fs::read(path) {
        if bytes.is_empty() {
            return None;
        }
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let mime = if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("png")).unwrap_or(false) {
            "image/png"
        } else {
            "image/jpeg"
        };
        Some(format!("data:{};base64,{}", mime, encoded))
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackSettings {
    pub volume: u32,
    pub is_muted: bool,
    pub speed: f32,
    pub is_paused: bool,
    pub pause_on_fullscreen: bool,
    pub pause_policy: u32, // 0: Always Play, 1: Fullscreen Game/App, 2: Maximized Window, 3: Any Window Focused, 4: Battery
    pub fill_mode: u32,    // 0: Stretch, 1: Keep Aspect / Fit, 2: Crop / Zoom / Expand, 3: Center
    pub pan_x: f32,        // -1.0 to 1.0 (Horizontal position offset)
    pub pan_y: f32,        // -1.0 to 1.0 (Vertical position offset)
    pub quality_level: u32, // 1: HD or lower, 2: FULLHD, 3: 2K, 4: 4K, 5: Native Source
    pub target_fps: u32,    // 0: Source/Native, 15, 24, 30, 60
    pub hardware_accel: bool,
    pub inactive_black_screen: bool,
    pub restart_on_resume: bool,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            volume: 0,
            is_muted: true,
            speed: 1.0,
            is_paused: false,
            pause_on_fullscreen: true,
            pause_policy: 1,
            fill_mode: 2,
            pan_x: 0.0,
            pan_y: 0.0,
            quality_level: 5,
            target_fps: 30,
            hardware_accel: true,
            inactive_black_screen: false,
            restart_on_resume: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub id: i32,
    pub name: String,
    pub resolution: String,
    pub current_wallpaper: Option<String>,
}

pub trait WallpaperBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn apply(&self, wallpaper: &WallpaperItem, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String>;
    fn pause(&self, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String>;
    fn resume(&self, screen: Option<i32>, settings: &PlaybackSettings, wallpaper: Option<&WallpaperItem>) -> Result<(), String>;
    fn set_volume(&self, volume: u32, muted: bool) -> Result<(), String>;
    fn set_speed(&self, speed: f32) -> Result<(), String>;
    fn set_geometry(&self, fill_mode: u32, pan_x: f32, pan_y: f32) -> Result<(), String> {
        let _ = (fill_mode, pan_x, pan_y);
        Ok(())
    }
    fn stop(&self, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopWallpaperBackup {
    pub screen: i32,
    pub plugin: String,
    pub config: std::collections::HashMap<String, String>,
}
