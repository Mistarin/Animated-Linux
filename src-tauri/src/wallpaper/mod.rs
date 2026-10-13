pub mod backend;
pub mod kde_plasma;
pub mod mpv_engine;

use backend::{PlaybackSettings, WallpaperBackend, WallpaperItem};
use kde_plasma::KdePlasmaBackend;
use mpv_engine::MpvBackend;
use std::sync::Mutex;
use std::path::{Path, PathBuf};

pub fn resolve_app_storage_dir() -> PathBuf {
    // Check if current directory or ancestors have "setup" or ("package.json" and "src-tauri")
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut check = Some(cwd.as_path());
    while let Some(dir) = check {
        if dir.join("setup").exists() || (dir.join("package.json").exists() && dir.join("src-tauri").exists()) {
            let app_wp = dir.join("wallpapers");
            let _ = std::fs::create_dir_all(&app_wp);
            let _ = std::fs::create_dir_all(app_wp.join("thumbnails"));
            return app_wp;
        }
        check = dir.parent();
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut check_exe = exe.parent();
        while let Some(dir) = check_exe {
            if dir.join("setup").exists() || (dir.join("package.json").exists() && dir.join("src-tauri").exists()) {
                let app_wp = dir.join("wallpapers");
                let _ = std::fs::create_dir_all(&app_wp);
                let _ = std::fs::create_dir_all(app_wp.join("thumbnails"));
                return app_wp;
            }
            check_exe = dir.parent();
        }
    }

    let default_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aetherpaper")
        .join("wallpapers");
    let _ = std::fs::create_dir_all(&default_dir);
    let _ = std::fs::create_dir_all(default_dir.join("thumbnails"));
    default_dir
}

pub struct WallpaperManager {
    plasma_backend: KdePlasmaBackend,
    mpv_backend: MpvBackend,
    pub current_settings: Mutex<PlaybackSettings>,
    pub active_wallpaper: Mutex<Option<WallpaperItem>>,
    storage_dir: PathBuf,
    library_path: PathBuf,
}

impl WallpaperManager {
    pub fn new() -> Self {
        let storage_dir = resolve_app_storage_dir();
        let library_path = storage_dir.join("library.json");

        // Migration: check if legacy ~/.config/aetherpaper/library.json exists and migrate
        if !library_path.exists() {
            if let Some(config_dir) = dirs::config_dir() {
                let legacy_dir = config_dir.join("aetherpaper");
                let legacy_lib = legacy_dir.join("library.json");
                if legacy_lib.exists() {
                    if let Ok(content) = std::fs::read_to_string(&legacy_lib) {
                        if let Ok(mut items) = serde_json::from_str::<Vec<WallpaperItem>>(&content) {
                            for item in &mut items {
                                let old_video = PathBuf::from(&item.file_path);
                                if old_video.exists() {
                                    let new_video = storage_dir.join(old_video.file_name().unwrap());
                                    let _ = std::fs::copy(&old_video, &new_video);
                                    item.file_path = new_video.to_string_lossy().to_string();
                                }
                                let old_thumb = PathBuf::from(&item.thumbnail_path);
                                if old_thumb.exists() {
                                    let new_thumb = storage_dir.join("thumbnails").join(old_thumb.file_name().unwrap());
                                    let _ = std::fs::copy(&old_thumb, &new_thumb);
                                    item.thumbnail_path = new_thumb.to_string_lossy().to_string();
                                }
                            }
                            let _ = std::fs::write(&library_path, serde_json::to_string_pretty(&items).unwrap_or_default());
                        }
                    }
                }
            }
        }

        Self {
            plasma_backend: KdePlasmaBackend::new(),
            mpv_backend: MpvBackend::new(),
            current_settings: Mutex::new(PlaybackSettings::default()),
            active_wallpaper: Mutex::new(None),
            storage_dir,
            library_path,
        }
    }

    pub fn get_storage_dir(&self) -> PathBuf {
        self.storage_dir.clone()
    }

    pub fn get_active_backend(&self) -> &dyn WallpaperBackend {
        if self.plasma_backend.is_available() {
            &self.plasma_backend
        } else {
            &self.mpv_backend
        }
    }

    pub fn apply_wallpaper(&self, wallpaper: WallpaperItem, screen: Option<i32>) -> Result<(), String> {
        let settings = self.current_settings.lock().unwrap().clone();
        let backend = self.get_active_backend();
        println!("[AetherPaper] Applying wallpaper via {}", backend.name());
        backend.apply(&wallpaper, screen, &settings)?;
        *self.active_wallpaper.lock().unwrap() = Some(wallpaper);
        Ok(())
    }

    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.is_paused = paused;
        let backend = self.get_active_backend();
        if paused {
            backend.pause(None)
        } else {
            backend.resume(None)
        }
    }

    pub fn set_volume(&self, volume: u32, muted: bool) -> Result<(), String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.volume = volume;
        settings.is_muted = muted;
        let backend = self.get_active_backend();
        backend.set_volume(volume, muted)
    }

    pub fn set_speed(&self, speed: f32) -> Result<(), String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.speed = speed;
        let backend = self.get_active_backend();
        backend.set_speed(speed)
    }

    pub fn get_settings(&self) -> PlaybackSettings {
        self.current_settings.lock().unwrap().clone()
    }

    pub fn update_performance_settings(
        &self,
        quality_level: u32,
        target_fps: u32,
        pause_on_fullscreen: bool,
        pause_policy: u32,
        hardware_accel: bool,
    ) -> Result<PlaybackSettings, String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.quality_level = quality_level.clamp(1, 5);
        settings.target_fps = target_fps;
        settings.pause_on_fullscreen = pause_on_fullscreen;
        settings.pause_policy = pause_policy;
        settings.hardware_accel = hardware_accel;
        let updated = settings.clone();
        drop(settings);

        // If a wallpaper is currently active, re-apply with new performance parameters
        let active = self.active_wallpaper.lock().unwrap().clone();
        if let Some(wp) = active {
            let backend = self.get_active_backend();
            let _ = backend.apply(&wp, None, &updated);
        }

        Ok(updated)
    }

    pub fn set_geometry(&self, fill_mode: u32, pan_x: f32, pan_y: f32) -> Result<(), String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.fill_mode = fill_mode;
        settings.pan_x = pan_x;
        settings.pan_y = pan_y;
        let backend = self.get_active_backend();
        backend.set_geometry(fill_mode, pan_x, pan_y)
    }

    pub fn stop(&self) -> Result<(), String> {
        let backend = self.get_active_backend();
        backend.stop(None)?;
        *self.active_wallpaper.lock().unwrap() = None;
        Ok(())
    }

    pub fn load_library(&self) -> Vec<WallpaperItem> {
        if let Ok(content) = std::fs::read_to_string(&self.library_path) {
            if let Ok(mut items) = serde_json::from_str::<Vec<WallpaperItem>>(&content) {
                for it in &mut items {
                    if it.thumbnail_data.is_none() {
                        it.thumbnail_data = backend::encode_thumbnail_base64(Path::new(&it.thumbnail_path));
                    }
                    // Scan and detect cached variants
                    let mut cached = Vec::new();
                    cached.push(5); // Q5 is original source
                    for q in 1..=4 {
                        let q_path = self.storage_dir.join(format!("{}_q{}.{}", it.id, q, it.format));
                        let q_mp4 = self.storage_dir.join(format!("{}_q{}.mp4", it.id, q));
                        if q_path.exists() || q_mp4.exists() {
                            cached.push(q);
                        }
                    }
                    cached.sort();
                    cached.dedup();
                    it.cached_qualities = cached;
                }
                return items;
            }
        }
        Vec::new()
    }

    pub fn save_library(&self, items: &[WallpaperItem]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
        std::fs::write(&self.library_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}
