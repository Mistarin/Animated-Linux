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
pub struct PlaybackSettings {
    pub volume: u32,
    pub is_muted: bool,
    pub speed: f32,
    pub is_paused: bool,
    pub pause_on_fullscreen: bool,
    pub fill_mode: u32, // 0: Stretch, 1: Keep Aspect Ratio, 2: Crop/Fill
    pub quality_level: u32, // 1: Max Performance, 2: Economy, 3: Balanced, 4: High Fidelity, 5: Raw Source
    pub target_fps: u32,    // 0: Source/Native, 15, 24, 30, 60
    pub hardware_accel: bool,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            volume: 0,
            is_muted: true,
            speed: 1.0,
            is_paused: false,
            pause_on_fullscreen: true,
            fill_mode: 2,
            quality_level: 3,
            target_fps: 30,
            hardware_accel: true,
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
    fn pause(&self, screen: Option<i32>) -> Result<(), String>;
    fn resume(&self, screen: Option<i32>) -> Result<(), String>;
    fn set_volume(&self, volume: u32, muted: bool) -> Result<(), String>;
    fn set_speed(&self, speed: f32) -> Result<(), String>;
    fn stop(&self, screen: Option<i32>) -> Result<(), String>;
}
