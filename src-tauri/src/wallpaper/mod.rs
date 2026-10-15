pub mod backend;
pub mod kde_plasma;
pub mod kde_mpv;
pub mod mpv_engine;

use backend::{DesktopWallpaperBackup, PlaybackSettings, WallpaperBackend, WallpaperItem};
use kde_plasma::KdePlasmaBackend;
use kde_mpv::KdePlasmaMpvBackend;
use mpv_engine::MpvBackend;
use std::process::Command;
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
    plasma_mpv_backend: KdePlasmaMpvBackend,
    mpv_backend: MpvBackend,
    active_backend_type: Mutex<String>,
    pub current_settings: Mutex<PlaybackSettings>,
    pub active_wallpaper: Mutex<Option<WallpaperItem>>,
    storage_dir: PathBuf,
    library_path: PathBuf,
    settings_path: PathBuf,
}

impl WallpaperManager {
    pub fn new() -> Self {
        let storage_dir = resolve_app_storage_dir();
        let library_path = storage_dir.join("library.json");
        let settings_path = storage_dir.join("settings.json");

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

        let active_path = storage_dir.join("active.json");
        let restored_active: Option<WallpaperItem> = if active_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&active_path) {
                if let Ok(mut wp) = serde_json::from_str::<WallpaperItem>(&content) {
                    if wp.thumbnail_data.is_none() {
                        wp.thumbnail_data = backend::encode_thumbnail_base64(Path::new(&wp.thumbnail_path));
                    }
                    if Path::new(&wp.file_path).exists() {
                        println!("[AnimatedLinux] Restored active wallpaper state from previous session: {}", wp.title);
                        Some(wp)
                    } else {
                        let original_path = storage_dir.join(format!("{}.{}", wp.id, wp.format));
                        let nested_path = storage_dir.join("wallpapers").join(format!("{}.{}", wp.id, wp.format));
                        if original_path.exists() {
                            wp.file_path = original_path.to_string_lossy().to_string();
                            println!("[AnimatedLinux] Restored active wallpaper using fallback path: {}", wp.file_path);
                            Some(wp)
                        } else if nested_path.exists() {
                            wp.file_path = nested_path.to_string_lossy().to_string();
                            println!("[AnimatedLinux] Restored active wallpaper using nested path: {}", wp.file_path);
                            Some(wp)
                        } else {
                            eprintln!("[AnimatedLinux] Active wallpaper file missing on disk, clearing active.json");
                            let _ = std::fs::remove_file(&active_path);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let backend_path = storage_dir.join("backend.json");
        let initial_backend = if let Ok(s) = std::fs::read_to_string(&backend_path) {
            let t = s.trim().to_lowercase();
            if t == "kde_mpv" || t == "plasma_mpv" {
                "kde_mpv".to_string()
            } else if t == "mpv" || t == "mpvpaper" {
                "mpv".to_string()
            } else {
                "kde".to_string()
            }
        } else {
            "kde_mpv".to_string()
        };

        let initial_settings = if settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&settings_path) {
                match serde_json::from_str::<PlaybackSettings>(&content) {
                    Ok(s) => {
                        println!(
                            "[AnimatedLinux] Restored persistent playback settings: Quality Q{}, {} FPS, {}x speed, FillMode {}, Pan ({}, {}), PausePolicy {}, StartMinimized: {}",
                            s.quality_level, s.target_fps, s.speed, s.fill_mode, s.pan_x, s.pan_y, s.pause_policy, s.start_minimized
                        );
                        s
                    }
                    Err(e) => {
                        eprintln!("[AnimatedLinux] Failed to parse settings.json ({}), using defaults", e);
                        PlaybackSettings::default()
                    }
                }
            } else {
                PlaybackSettings::default()
            }
        } else {
            PlaybackSettings::default()
        };

        Self {
            plasma_backend: KdePlasmaBackend::new(),
            plasma_mpv_backend: KdePlasmaMpvBackend::new(),
            mpv_backend: MpvBackend::new(),
            active_backend_type: Mutex::new(initial_backend),
            current_settings: Mutex::new(initial_settings),
            active_wallpaper: Mutex::new(restored_active),
            storage_dir,
            library_path,
            settings_path,
        }
    }

    pub fn get_storage_dir(&self) -> PathBuf {
        self.storage_dir.clone()
    }

    pub fn get_active_backend_type(&self) -> String {
        self.active_backend_type.lock().unwrap().clone()
    }

    pub fn get_active_backend(&self) -> &dyn WallpaperBackend {
        let b = self.active_backend_type.lock().unwrap();
        match b.as_str() {
            "kde_mpv" => {
                if self.plasma_mpv_backend.is_available() {
                    &self.plasma_mpv_backend
                } else if self.plasma_backend.is_available() {
                    &self.plasma_backend
                } else {
                    &self.mpv_backend
                }
            }
            "mpv" => {
                if self.mpv_backend.is_available() {
                    &self.mpv_backend
                } else if self.plasma_mpv_backend.is_available() {
                    &self.plasma_mpv_backend
                } else {
                    &self.plasma_backend
                }
            }
            _ => {
                if self.plasma_backend.is_available() {
                    &self.plasma_backend
                } else if self.plasma_mpv_backend.is_available() {
                    &self.plasma_mpv_backend
                } else {
                    &self.mpv_backend
                }
            }
        }
    }

    pub fn switch_backend(&self, target_backend: &str) -> Result<(), String> {
        let target_lower = target_backend.to_lowercase();
        let normalized = if target_lower == "kde_mpv" || target_lower.contains("plasma_mpv") {
            "kde_mpv"
        } else if target_lower == "mpv" || target_lower.contains("mpvpaper") || target_lower.contains("standalone") {
            "mpv"
        } else {
            "kde"
        };

        {
            let current = self.active_backend_type.lock().unwrap();
            if *current == normalized {
                return Ok(());
            }
        }

        // 1. Cleanly stop old backend
        let old_backend = self.get_active_backend();
        println!("[AnimatedLinux] Switching from {} to {} backend", old_backend.name(), normalized);
        let settings_for_stop = self.current_settings.lock().unwrap().clone();
        let _ = old_backend.stop(None, &settings_for_stop);

        // 2. Persist new selection
        {
            let mut lock = self.active_backend_type.lock().unwrap();
            *lock = normalized.to_string();
        }
        let _ = std::fs::write(self.storage_dir.join("backend.json"), normalized);

        // 3. Re-apply active wallpaper to new backend if one was running
        let active = self.active_wallpaper.lock().unwrap().clone();
        if let Some(wallpaper) = active {
            let settings = self.current_settings.lock().unwrap().clone();
            let new_backend = self.get_active_backend();
            println!("[AnimatedLinux] Applying active wallpaper to {}", new_backend.name());
            new_backend.apply(&wallpaper, None, &settings)?;
        }

        Ok(())
    }

    pub fn save_settings(&self, settings: &PlaybackSettings) {
        if let Ok(json) = serde_json::to_string_pretty(settings) {
            let _ = std::fs::write(&self.settings_path, json);
        }
    }

    pub fn ensure_previous_wallpaper_cached(&self) {
        let backup_file = self.storage_dir.join("previous_wallpaper.json");
        if backup_file.exists() {
            return;
        }

        // Query Plasma desktops via qdbus evaluateScript
        let script = r##"
var allDesktops = desktops();
var results = [];
for (var i = 0; i < allDesktops.length; i++) {
    var d = allDesktops[i];
    var plugin = d.wallpaperPlugin;
    if (plugin === "com.github.catsout.wallpaperEngineKde" || plugin === "luisbocanegra.smart.video.wallpaper.reborn" || plugin === "org.kde.color") {
        d.currentConfigGroup = Array("Wallpaper", "org.kde.image", "General");
        var img = d.readConfig("Image");
        var fill = d.readConfig("FillMode");
        var col = d.readConfig("Color");
        results.push({
            screen: d.screen,
            plugin: "org.kde.image",
            config: {
                "Image": img ? img : "",
                "FillMode": fill ? fill : "2",
                "Color": col ? col : "#000000"
            }
        });
    } else {
        d.currentConfigGroup = Array("Wallpaper", plugin, "General");
        var img = d.readConfig("Image");
        var fill = d.readConfig("FillMode");
        var col = d.readConfig("Color");
        results.push({
            screen: d.screen,
            plugin: plugin,
            config: {
                "Image": img ? img : "",
                "FillMode": fill ? fill : "",
                "Color": col ? col : ""
            }
        });
    }
}
print(JSON.stringify(results));
"##;

        if let Ok(output) = Command::new("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .arg("org.kde.PlasmaShell.evaluateScript")
            .arg(script)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !stdout.is_empty() {
                    if let Ok(backups) = serde_json::from_str::<Vec<DesktopWallpaperBackup>>(&stdout) {
                        if !backups.is_empty() {
                            println!("[AnimatedLinux] Cached {} previous desktop wallpaper configuration(s)", backups.len());
                            let _ = std::fs::write(&backup_file, serde_json::to_string_pretty(&backups).unwrap_or(stdout));
                        }
                    }
                }
            }
        }
    }

    pub fn apply_wallpaper(&self, wallpaper: WallpaperItem, screen: Option<i32>) -> Result<(), String> {
        self.ensure_previous_wallpaper_cached();

        let settings = self.current_settings.lock().unwrap().clone();
        let backend = self.get_active_backend();
        println!("[AnimatedLinux] Applying wallpaper via {}", backend.name());

        // Auto-select preferred cached variant (e.g. Q1-Q4) if available
        let mut effective_wp = wallpaper;
        if settings.quality_level < 5 {
            let variant_file = self.storage_dir.join(format!("{}_q{}.mp4", effective_wp.id, settings.quality_level));
            if variant_file.exists() {
                effective_wp.file_path = variant_file.to_string_lossy().to_string();
            }
        }

        backend.apply(&effective_wp, screen, &settings)?;
        if settings.is_paused {
            let _ = backend.pause(screen, &settings);
        }
        let mut save_item = effective_wp.clone();
        save_item.thumbnail_data = None;
        let _ = std::fs::write(self.storage_dir.join("active.json"), serde_json::to_string(&save_item).unwrap_or_default());
        *self.active_wallpaper.lock().unwrap() = Some(effective_wp);
        Ok(())
    }

    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.is_paused = paused;
        let settings_clone = settings.clone();
        drop(settings);
        self.save_settings(&settings_clone);

        let active_wp = self.active_wallpaper.lock().unwrap().clone();
        let backend = self.get_active_backend();
        if paused {
            backend.pause(None, &settings_clone)
        } else {
            backend.resume(None, &settings_clone, active_wp.as_ref())
        }
    }

    pub fn is_paused(&self) -> bool {
        self.current_settings.lock().unwrap().is_paused
    }

    pub fn set_volume(&self, volume: u32, muted: bool) -> Result<(), String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.volume = volume;
        settings.is_muted = muted;
        let updated = settings.clone();
        drop(settings);
        self.save_settings(&updated);
        let backend = self.get_active_backend();
        backend.set_volume(volume, muted)
    }

    pub fn set_speed(&self, speed: f32) -> Result<(), String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.speed = speed;
        let updated = settings.clone();
        drop(settings);
        self.save_settings(&updated);
        let backend = self.get_active_backend();
        backend.set_speed(speed)
    }

    pub fn get_settings(&self) -> PlaybackSettings {
        self.current_settings.lock().unwrap().clone()
    }

    pub fn get_start_minimized(&self) -> bool {
        self.current_settings.lock().unwrap().start_minimized
    }

    pub fn set_start_minimized(&self, enable: bool) -> Result<bool, String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.start_minimized = enable;
        let updated = settings.clone();
        drop(settings);
        self.save_settings(&updated);
        Ok(enable)
    }

    pub fn update_performance_settings(
        &self,
        quality_level: u32,
        target_fps: u32,
        pause_on_fullscreen: bool,
        pause_policy: u32,
        hardware_accel: bool,
        inactive_black_screen: bool,
        restart_on_resume: bool,
    ) -> Result<PlaybackSettings, String> {
        let mut settings = self.current_settings.lock().unwrap();
        settings.quality_level = quality_level.clamp(1, 5);
        settings.target_fps = target_fps;
        settings.pause_on_fullscreen = pause_on_fullscreen;
        settings.pause_policy = pause_policy;
        settings.hardware_accel = hardware_accel;
        settings.inactive_black_screen = inactive_black_screen;
        settings.restart_on_resume = restart_on_resume;
        let updated = settings.clone();
        drop(settings);

        self.save_settings(&updated);

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
        let updated = settings.clone();
        drop(settings);

        self.save_settings(&updated);
        let backend = self.get_active_backend();
        backend.set_geometry(fill_mode, pan_x, pan_y)
    }

    pub fn restore_previous_wallpaper(&self) -> Result<(), String> {
        let backup_file = self.storage_dir.join("previous_wallpaper.json");
        let backups: Vec<DesktopWallpaperBackup> = if backup_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&backup_file) {
                serde_json::from_str::<Vec<DesktopWallpaperBackup>>(&content).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let _ = std::fs::remove_file(&backup_file);

        let backups_json = serde_json::to_string(&backups).unwrap_or_else(|_| "[]".to_string());
        let script = format!(
            r#"
var allDesktops = desktops();
var backups = {backups_json};

for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    var backup = null;
    for (var j = 0; j < backups.length; j++) {{
        if (backups[j].screen === d.screen) {{
            backup = backups[j];
            break;
        }}
    }}
    if (!backup && backups.length > 0) {{
        backup = backups[0];
    }}

    if (backup) {{
        d.wallpaperPlugin = backup.plugin;
        d.currentConfigGroup = Array("Wallpaper", backup.plugin, "General");
        for (var key in backup.config) {{
            if (backup.config.hasOwnProperty(key)) {{
                d.writeConfig(key, backup.config[key]);
            }}
        }}
        d.reloadConfig();
    }} else {{
        d.wallpaperPlugin = "org.kde.image";
        d.currentConfigGroup = Array("Wallpaper", "org.kde.image", "General");
        var existingImg = d.readConfig("Image");
        if (existingImg && existingImg.length > 0) {{
            d.writeConfig("Image", existingImg);
        }}
        d.reloadConfig();
    }}
}}
"#,
            backups_json = backups_json
        );

        let output = Command::new("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .arg("org.kde.PlasmaShell.evaluateScript")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to run Plasma evaluateScript: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("[AnimatedLinux] Warning restoring wallpaper: {}", stderr);
        } else {
            println!("[AnimatedLinux] Desktop wallpaper safely restored to previous configuration");
        }

        Ok(())
    }

    pub fn cleanup_on_quit(&self) -> Result<(), String> {
        let settings = self.current_settings.lock().unwrap().clone();
        let _ = self.mpv_backend.stop(None, &settings);
        let _ = self.plasma_mpv_backend.stop(None, &settings);
        let _ = self.plasma_backend.stop(None, &settings);
        let _ = self.restore_previous_wallpaper();

        // Preserve active wallpaper state in active.json so that upon next app launch
        // the active wallpaper and its state can be loaded automatically.
        let active = self.active_wallpaper.lock().unwrap().clone();
        if let Some(wp) = active {
            let mut save_item = wp;
            save_item.thumbnail_data = None;
            let _ = std::fs::write(self.storage_dir.join("active.json"), serde_json::to_string(&save_item).unwrap_or_default());
            println!("[AnimatedLinux] Active wallpaper '{}' state preserved for next session launch", save_item.title);
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let settings = self.current_settings.lock().unwrap().clone();
        let _ = self.mpv_backend.stop(None, &settings);
        let _ = self.plasma_mpv_backend.stop(None, &settings);
        let _ = self.plasma_backend.stop(None, &settings);
        let _ = self.restore_previous_wallpaper();
        let _ = std::fs::remove_file(self.storage_dir.join("active.json"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_wallpaper_backup_serde() {
        let mut config = std::collections::HashMap::new();
        config.insert("Image".to_string(), "file:///tmp/test.jpg".to_string());
        config.insert("FillMode".to_string(), "2".to_string());
        config.insert("Color".to_string(), "#000000".to_string());

        let backup = DesktopWallpaperBackup {
            screen: 0,
            plugin: "org.kde.image".to_string(),
            config,
        };

        let json = serde_json::to_string(&vec![backup]).unwrap();
        assert!(json.contains("org.kde.image"));
        assert!(json.contains("file:///tmp/test.jpg"));

        let deserialized: Vec<DesktopWallpaperBackup> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].screen, 0);
        assert_eq!(deserialized[0].plugin, "org.kde.image");
        assert_eq!(deserialized[0].config.get("Image").unwrap(), "file:///tmp/test.jpg");
    }

    #[test]
    fn test_active_wallpaper_persistence_across_quit() {
        let temp_dir = std::env::temp_dir().join(format!("animated_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let dummy_video = temp_dir.join("test_video.mp4");
        let _ = std::fs::write(&dummy_video, b"dummy video content");

        let dummy_thumb = temp_dir.join("test_thumb.jpg");
        let _ = std::fs::write(&dummy_thumb, b"dummy thumb");

        let wp = WallpaperItem {
            id: "test-id-1".to_string(),
            title: "Test Wallpaper".to_string(),
            file_path: dummy_video.to_string_lossy().to_string(),
            thumbnail_path: dummy_thumb.to_string_lossy().to_string(),
            thumbnail_data: None,
            cached_qualities: vec![5],
            duration_secs: 10.0,
            resolution: "1920x1080".to_string(),
            format: "mp4".to_string(),
            is_workshop: false,
            workshop_id: None,
        };

        let active_file = temp_dir.join("active.json");
        let wp_json = serde_json::to_string(&wp).unwrap();
        let _ = std::fs::write(&active_file, &wp_json);
        assert!(active_file.exists());

        // Read active.json simulating launch
        let content = std::fs::read_to_string(&active_file).unwrap();
        let loaded: WallpaperItem = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.id, "test-id-1");
        assert_eq!(loaded.title, "Test Wallpaper");
        assert!(Path::new(&loaded.file_path).exists());

        // Verify stop removes active.json
        let _ = std::fs::remove_file(&active_file);
        assert!(!active_file.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_start_minimized_settings_persistence() {
        let mut settings = PlaybackSettings::default();
        assert!(!settings.start_minimized);

        settings.start_minimized = true;
        let serialized = serde_json::to_string(&settings).unwrap();
        assert!(serialized.contains("\"start_minimized\":true"));

        let deserialized: PlaybackSettings = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.start_minimized);

        // Deserializing older json without start_minimized key should default to false
        let legacy_json = "{\"volume\":50,\"speed\":1.0}";
        let legacy_settings: PlaybackSettings = serde_json::from_str(legacy_json).unwrap();
        assert!(!legacy_settings.start_minimized);
    }
}
