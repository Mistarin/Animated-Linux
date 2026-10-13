// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod wallpaper;
mod steam;
mod monitors;
mod performance;
mod tray;

use wallpaper::{backend::*, WallpaperManager};
use steam::{SteamManager, SteamStatus, WorkshopItem, WorkshopManager, SteamShortcutInstaller};
use monitors::MonitorDetector;
use tauri::State;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::process::Command;

struct AppState {
    wallpaper_mgr: Arc<WallpaperManager>,
    steam_mgr: Arc<SteamManager>,
    workshop_mgr: WorkshopManager,
}

#[tauri::command]
fn get_wallpapers(state: State<'_, AppState>) -> Vec<WallpaperItem> {
    state.wallpaper_mgr.load_library()
}

fn import_wallpaper_internal(state: &AppState, source_path: &str) -> Result<WallpaperItem, String> {
    let src = Path::new(source_path);
    if !src.exists() {
        return Err(format!("Source file does not exist: {}", source_path));
    }

    let file_stem = src.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let ext = src.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    let id = format!("wp_{}", uuid_simple());

    let wallpapers_dir = state.wallpaper_mgr.get_storage_dir();
    let thumbs_dir = wallpapers_dir.join("thumbnails");
    let _ = std::fs::create_dir_all(&wallpapers_dir);
    let _ = std::fs::create_dir_all(&thumbs_dir);

    let dest_video = wallpapers_dir.join(format!("{}.{}", id, ext));
    let dest_thumb = thumbs_dir.join(format!("{}.jpg", id));

    // Copy video file
    std::fs::copy(src, &dest_video).map_err(|e| format!("Failed to copy video: {}", e))?;

    // Extract thumbnail using ffmpeg
    let _ = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss").arg("00:00:01")
        .arg("-i").arg(&dest_video)
        .arg("-vframes").arg("1")
        .arg("-q:v").arg("2")
        .arg(&dest_thumb)
        .output();

    // Query resolution and duration using ffprobe if available
    let resolution = get_video_resolution(&dest_video).unwrap_or_else(|| "1920x1080".to_string());
    let duration = get_video_duration(&dest_video).unwrap_or(15.0);
    let thumbnail_data = encode_thumbnail_base64(&dest_thumb);

    let item = WallpaperItem {
        id,
        title: file_stem,
        file_path: dest_video.to_string_lossy().to_string(),
        thumbnail_path: dest_thumb.to_string_lossy().to_string(),
        thumbnail_data,
        cached_qualities: vec![5],
        duration_secs: duration,
        resolution,
        format: ext,
        is_workshop: false,
        workshop_id: None,
    };

    let mut library = state.wallpaper_mgr.load_library();
    library.push(item.clone());
    state.wallpaper_mgr.save_library(&library)?;

    Ok(item)
}

#[tauri::command]
fn import_wallpaper(state: State<'_, AppState>, source_path: String) -> Result<WallpaperItem, String> {
    import_wallpaper_internal(&state, &source_path)
}

#[tauri::command]
fn select_and_import_videos(state: State<'_, AppState>) -> Result<Vec<WallpaperItem>, String> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut paths_to_import: Vec<String> = Vec::new();

    // 1. Try kdialog (native KDE file dialog)
    if let Ok(out) = Command::new("kdialog")
        .arg("--getopenfilename")
        .arg("--multiple")
        .arg(&home)
        .arg("Video Files (*.mp4 *.webm *.mkv *.gif *.mov *.avi *.flv *.m4v)")
        .output()
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    paths_to_import.push(trimmed.to_string());
                }
            }
        }
    }

    // 2. Try zenity fallback
    if paths_to_import.is_empty() {
        if let Ok(out) = Command::new("zenity")
            .arg("--file-selection")
            .arg("--multiple")
            .arg("--separator=|")
            .arg("--title=Select Animated Wallpaper")
            .arg("--file-filter=Video Files (*.mp4, *.webm, *.mkv, *.gif, *.mov) | *.mp4 *.webm *.mkv *.gif *.mov *.avi *.flv *.m4v")
            .output()
        {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for part in stdout.trim().split('|') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        paths_to_import.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    let mut imported = Vec::new();
    for p in paths_to_import {
        if let Ok(item) = import_wallpaper_internal(&state, &p) {
            imported.push(item);
        }
    }

    Ok(imported)
}

#[tauri::command]
fn delete_wallpaper(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut library = state.wallpaper_mgr.load_library();
    if let Some(pos) = library.iter().position(|w| w.id == id) {
        let item = library.remove(pos);
        let _ = std::fs::remove_file(&item.file_path);
        let _ = std::fs::remove_file(&item.thumbnail_path);
        state.wallpaper_mgr.save_library(&library)?;
    }
    Ok(())
}

#[tauri::command]
fn rename_wallpaper(state: State<'_, AppState>, id: String, new_title: String) -> Result<WallpaperItem, String> {
    let mut library = state.wallpaper_mgr.load_library();
    let trimmed = new_title.trim();
    if trimmed.is_empty() {
        return Err("Title cannot be empty".to_string());
    }
    if let Some(pos) = library.iter().position(|w| w.id == id) {
        library[pos].title = trimmed.to_string();
        let updated = library[pos].clone();
        state.wallpaper_mgr.save_library(&library)?;

        let mut active_lock = state.wallpaper_mgr.active_wallpaper.lock().unwrap();
        if let Some(active) = &*active_lock {
            if active.id == id {
                *active_lock = Some(updated.clone());
                state.steam_mgr.set_wallpaper_presence(&updated.title);
            }
        }
        return Ok(updated);
    }
    Err("Wallpaper not found".to_string())
}

#[tauri::command]
fn apply_wallpaper(state: State<'_, AppState>, wallpaper: WallpaperItem, screen: Option<i32>) -> Result<(), String> {
    state.wallpaper_mgr.apply_wallpaper(wallpaper.clone(), screen)?;
    state.steam_mgr.set_wallpaper_presence(&wallpaper.title);
    Ok(())
}

#[tauri::command]
fn stop_wallpaper(state: State<'_, AppState>) -> Result<(), String> {
    state.wallpaper_mgr.stop()
}

#[tauri::command]
fn set_playback_state(
    state: State<'_, AppState>,
    is_paused: bool,
    volume: u32,
    is_muted: bool,
    speed: f32,
) -> Result<(), String> {
    state.wallpaper_mgr.set_paused(is_paused)?;
    state.wallpaper_mgr.set_volume(volume, is_muted)?;
    state.wallpaper_mgr.set_speed(speed)?;
    Ok(())
}

#[tauri::command]
fn get_performance_settings(state: State<'_, AppState>) -> PlaybackSettings {
    state.wallpaper_mgr.get_settings()
}

#[tauri::command]
fn update_performance_settings(
    state: State<'_, AppState>,
    quality_level: u32,
    target_fps: u32,
    pause_on_fullscreen: bool,
    pause_policy: u32,
    hardware_accel: bool,
) -> Result<PlaybackSettings, String> {
    state.wallpaper_mgr.update_performance_settings(
        quality_level,
        target_fps,
        pause_on_fullscreen,
        pause_policy,
        hardware_accel,
    )
}

#[tauri::command]
fn set_video_geometry(
    state: State<'_, AppState>,
    fill_mode: u32,
    pan_x: f32,
    pan_y: f32,
) -> Result<(), String> {
    state.wallpaper_mgr.set_geometry(fill_mode, pan_x, pan_y)
}

#[tauri::command]
fn switch_wallpaper_quality(
    state: State<'_, AppState>,
    id: String,
    quality_level: u32,
) -> Result<WallpaperItem, String> {
    let mut library = state.wallpaper_mgr.load_library();
    let pos = library
        .iter()
        .position(|w| w.id == id)
        .ok_or_else(|| "Wallpaper not found".to_string())?;

    let item = &mut library[pos];
    let wallpapers_dir = state.wallpaper_mgr.get_storage_dir();
    let orig_backup = wallpapers_dir.join(format!("{}_source.{}", id, item.format));

    let variant_path = if quality_level == 5 {
        if orig_backup.exists() {
            orig_backup
        } else {
            PathBuf::from(&item.file_path)
        }
    } else {
        wallpapers_dir.join(format!("{}_q{}.mp4", id, quality_level))
    };

    if !variant_path.exists() {
        return Err(format!("Variant Q{} is not cached yet", quality_level));
    }

    item.file_path = variant_path.to_string_lossy().to_string();
    if let Some(res) = get_video_resolution(&variant_path) {
        item.resolution = res;
    }
    if let Some(dur) = get_video_duration(&variant_path) {
        item.duration_secs = dur;
    }

    let updated = item.clone();
    state.wallpaper_mgr.save_library(&library)?;

    let active_id = {
        let active = state.wallpaper_mgr.active_wallpaper.lock().unwrap();
        active.as_ref().map(|w| w.id.clone())
    };
    if let Some(active_id) = active_id {
        if active_id == id {
            let _ = state.wallpaper_mgr.apply_wallpaper(updated.clone(), None);
        }
    }

    Ok(updated)
}

#[tauri::command]
fn compress_wallpaper(
    state: State<'_, AppState>,
    id: String,
    quality_level: u32,
    target_fps: u32,
) -> Result<WallpaperItem, String> {
    let mut library = state.wallpaper_mgr.load_library();
    let pos = library
        .iter()
        .position(|w| w.id == id)
        .ok_or_else(|| "Wallpaper not found".to_string())?;

    let item = &mut library[pos];
    let current_video_path = PathBuf::from(&item.file_path);
    if !current_video_path.exists() {
        return Err("Source video file does not exist".to_string());
    }

    let wallpapers_dir = state.wallpaper_mgr.get_storage_dir();
    let orig_backup = wallpapers_dir.join(format!("{}_source.{}", id, item.format));

    // Preserve original source untouched
    if !orig_backup.exists() {
        let _ = std::fs::copy(&current_video_path, &orig_backup);
    }

    let input_source = if orig_backup.exists() {
        orig_backup.clone()
    } else {
        current_video_path.clone()
    };

    let target_variant_path = if quality_level == 5 {
        orig_backup.clone()
    } else {
        wallpapers_dir.join(format!("{}_q{}.mp4", id, quality_level))
    };

    if quality_level == 5 {
        item.file_path = orig_backup.to_string_lossy().to_string();
    } else {
        // Q4 is 4K (2160p), Q3 is 2K (1440p), Q2 is FULLHD (1080p), Q1 is HD or lower (720p)
        let (max_height, crf) = match quality_level {
            1 => (720, "28"),
            2 => (1080, "24"),
            3 => (1440, "21"),
            4 => (2160, "18"),
            _ => (1080, "24"),
        };

        let fps = if target_fps > 0 {
            target_fps
        } else {
            match quality_level {
                1 => 24,
                2 => 30,
                3 => 60,
                4 => 60,
                _ => 30,
            }
        };

        let vf = format!("scale=-2:{},fps={}", max_height, fps);
        let temp_output = wallpapers_dir.join(format!("{}_temp_q{}.mp4", id, quality_level));

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i").arg(&input_source)
            .arg("-vf").arg(&vf)
            .arg("-c:v").arg("libx264")
            .arg("-preset").arg("veryfast")
            .arg("-crf").arg(crf)
            .arg("-an")
            .arg(&temp_output)
            .status()
            .map_err(|e| format!("Failed to invoke ffmpeg: {}", e))?;

        if !status.success() {
            return Err("ffmpeg failed to compress video".to_string());
        }

        let _ = std::fs::rename(&temp_output, &target_variant_path);
        item.file_path = target_variant_path.to_string_lossy().to_string();
    }

    if !item.cached_qualities.contains(&quality_level) {
        item.cached_qualities.push(quality_level);
        item.cached_qualities.sort();
        item.cached_qualities.dedup();
    }

    if let Some(res) = get_video_resolution(&target_variant_path) {
        item.resolution = res;
    }
    if let Some(dur) = get_video_duration(&target_variant_path) {
        item.duration_secs = dur;
    }
    let thumb_path = PathBuf::from(&item.thumbnail_path);
    if thumb_path.exists() {
        item.thumbnail_data = encode_thumbnail_base64(&thumb_path);
    }

    let updated = item.clone();
    state.wallpaper_mgr.save_library(&library)?;

    // If active, re-apply
    let active_id = {
        let active = state.wallpaper_mgr.active_wallpaper.lock().unwrap();
        active.as_ref().map(|w| w.id.clone())
    };
    if let Some(active_id) = active_id {
        if active_id == id {
            let _ = state.wallpaper_mgr.apply_wallpaper(updated.clone(), None);
        }
    }

    Ok(updated)
}

#[tauri::command]
fn get_monitors() -> Vec<ScreenInfo> {
    MonitorDetector::detect_monitors()
}

#[tauri::command]
fn get_steam_status(state: State<'_, AppState>) -> SteamStatus {
    state.steam_mgr.get_status()
}

#[tauri::command]
fn get_workshop_items(state: State<'_, AppState>) -> Vec<WorkshopItem> {
    state.workshop_mgr.get_community_items()
}

#[tauri::command]
fn add_to_steam_client() -> Result<String, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let accounts_updated = SteamShortcutInstaller::install_to_steam(&current_exe, "AetherPaper", &current_dir)?;
    Ok(format!("Successfully registered AetherPaper to {} Steam account(s)! Restart Steam to see it in your library.", accounts_updated))
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    format!("{:x}", now)
}

fn get_video_resolution(path: &Path) -> Option<String> {
    let out = Command::new("ffprobe")
        .arg("-v").arg("error")
        .arg("-select_streams").arg("v:0")
        .arg("-show_entries").arg("stream=width,height")
        .arg("-of").arg("csv=s=x:p=0")
        .arg(path)
        .output()
        .ok()?;
    if out.status.success() {
        let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !res.is_empty() {
            return Some(res);
        }
    }
    None
}

fn get_video_duration(path: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .arg("-v").arg("error")
        .arg("-show_entries").arg("format=duration")
        .arg("-of").arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .ok()?;
    if out.status.success() {
        let dur_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return dur_str.parse::<f64>().ok();
    }
    None
}

fn main() {
    let steam_mgr = SteamManager::new(480);
    let wallpaper_mgr = Arc::new(WallpaperManager::new());
    let workshop_mgr = WorkshopManager::new();

    let app_state = AppState {
        wallpaper_mgr: wallpaper_mgr.clone(),
        steam_mgr: steam_mgr.clone(),
        workshop_mgr,
    };

    tauri::Builder::default()
        .setup(move |app| {
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("[Tray] Warning: could not initialize system tray: {}", e);
            }

            // Start fullscreen game detection monitor task
            let wp_mgr = wallpaper_mgr.clone();
            tauri::async_runtime::spawn(async move {
                let mut was_game_fullscreen = false;
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    let has_active_wp = wp_mgr.active_wallpaper.lock().unwrap().is_some();
                    if has_active_wp {
                        let pause_on_fullscreen = wp_mgr.current_settings.lock().unwrap().pause_on_fullscreen;
                        if pause_on_fullscreen {
                            let is_game = performance::GameDetector::is_fullscreen_app_active();
                            if is_game && !was_game_fullscreen {
                                println!("[AetherPaper] Fullscreen game detected -> Pausing wallpaper");
                                let _ = wp_mgr.set_paused(true);
                                was_game_fullscreen = true;
                            } else if !is_game && was_game_fullscreen {
                                println!("[AetherPaper] Fullscreen game exited -> Resuming wallpaper");
                                let _ = wp_mgr.set_paused(false);
                                was_game_fullscreen = false;
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_wallpapers,
            import_wallpaper,
            select_and_import_videos,
            delete_wallpaper,
            rename_wallpaper,
            apply_wallpaper,
            stop_wallpaper,
            set_playback_state,
            get_performance_settings,
            update_performance_settings,
            compress_wallpaper,
            switch_wallpaper_quality,
            set_video_geometry,
            get_monitors,
            get_steam_status,
            get_workshop_items,
            add_to_steam_client
        ])
        .run(tauri::generate_context!())
        .expect("error while running AetherPaper tauri application");
}
