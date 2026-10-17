#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod wallpaper;
mod steam;
mod monitors;
mod performance;
mod tray;
mod request_manager;

use wallpaper::{backend::*, WallpaperManager};
use steam::{SteamManager, SteamStatus, WorkshopItem, WorkshopManager, SteamShortcutInstaller};
use monitors::MonitorDetector;
use request_manager::RequestManager;
use tauri::State;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn new_host_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    // Restore host environment if needed so external binaries (mpv, ffmpeg, qdbus) load host GPU drivers
    if let Ok(orig_ld) = std::env::var("LD_LIBRARY_PATH_ORIG") {
        cmd.env("LD_LIBRARY_PATH", orig_ld);
    } else if let Ok(orig_ld) = std::env::var("ORIG_LD_LIBRARY_PATH") {
        cmd.env("LD_LIBRARY_PATH", orig_ld);
    } else if let Ok(appdir) = std::env::var("APPDIR") {
        if let Ok(cur_ld) = std::env::var("LD_LIBRARY_PATH") {
            let filtered: Vec<&str> = cur_ld
                .split(':')
                .filter(|p| !p.contains(&appdir) && !p.contains(".mount_"))
                .collect();
            if filtered.is_empty() {
                cmd.env_remove("LD_LIBRARY_PATH");
            } else {
                cmd.env("LD_LIBRARY_PATH", filtered.join(":"));
            }
        }
    } else if let Ok(cur_ld) = std::env::var("LD_LIBRARY_PATH") {
        if cur_ld.contains(".mount_") {
            let filtered: Vec<&str> = cur_ld
                .split(':')
                .filter(|p| !p.contains(".mount_"))
                .collect();
            if filtered.is_empty() {
                cmd.env_remove("LD_LIBRARY_PATH");
            } else {
                cmd.env("LD_LIBRARY_PATH", filtered.join(":"));
            }
        }
    }

    if let Ok(preload) = std::env::var("LD_PRELOAD") {
        if preload.contains(".mount_") {
            cmd.env_remove("LD_PRELOAD");
        }
    }

    for driver_var in &["LIBVA_DRIVERS_PATH", "LIBGL_DRIVERS_PATH", "__EGL_VENDOR_LIBRARY_DIRS"] {
        if let Ok(val) = std::env::var(driver_var) {
            if val.contains(".mount_") {
                cmd.env_remove(driver_var);
            }
        }
    }

    cmd
}


pub struct AppState {
    pub wallpaper_mgr: Arc<WallpaperManager>,
    pub steam_mgr: Arc<SteamManager>,
    pub workshop_mgr: WorkshopManager,
    pub request_manager: Arc<RequestManager>,
    pub caching_progress: Arc<std::sync::Mutex<Option<CachingProgressPayload>>>,
}

#[tauri::command]
fn get_wallpapers(state: State<'_, AppState>) -> Vec<WallpaperItem> {
    state.wallpaper_mgr.load_library()
}

fn import_single_file_to_storage(wp_mgr: &WallpaperManager, source_path: &str) -> Result<WallpaperItem, String> {
    let src = Path::new(source_path);
    if !src.exists() {
        return Err(format!("Source file does not exist: {}", source_path));
    }

    let file_stem = src.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let ext = src.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    let id = format!("wp_{}", uuid_simple());

    let wallpapers_dir = wp_mgr.get_storage_dir();
    let thumbs_dir = wallpapers_dir.join("thumbnails");
    let _ = std::fs::create_dir_all(&wallpapers_dir);
    let _ = std::fs::create_dir_all(&thumbs_dir);

    let dest_video = wallpapers_dir.join(format!("{}.{}", id, ext));
    let dest_thumb = thumbs_dir.join(format!("{}.jpg", id));

    // Copy video file
    std::fs::copy(src, &dest_video).map_err(|e| format!("Failed to copy video: {}", e))?;

    // Extract thumbnail using ffmpeg with host environment
    let _ = new_host_command("ffmpeg")
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

    Ok(WallpaperItem {
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
    })
}

fn import_wallpaper_internal(wp_mgr: &WallpaperManager, source_path: &str) -> Result<WallpaperItem, String> {
    let item = import_single_file_to_storage(wp_mgr, source_path)?;
    let mut library = wp_mgr.load_library();
    library.push(item.clone());
    wp_mgr.save_library(&library)?;
    Ok(item)
}

#[tauri::command]
async fn import_wallpaper(state: State<'_, AppState>, source_path: String) -> Result<WallpaperItem, String> {
    let wp_mgr = state.wallpaper_mgr.clone();
    tokio::task::spawn_blocking(move || {
        import_wallpaper_internal(&wp_mgr, &source_path)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
async fn get_quick_preview(source_path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let p = Path::new(&source_path);
        if !p.exists() {
            return Err("Source file does not exist".to_string());
        }

        // Fast extraction of preview frame via ffmpeg directly to stdout
        let out = new_host_command("ffmpeg")
            .arg("-y")
            .arg("-ss").arg("00:00:01")
            .arg("-i").arg(&source_path)
            .arg("-vframes").arg("1")
            .arg("-s").arg("320x180")
            .arg("-f").arg("image2")
            .arg("-c:v").arg("mjpeg")
            .arg("-")
            .output()
            .map_err(|e| e.to_string())?;

        if out.status.success() && !out.stdout.is_empty() {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&out.stdout);
            Ok(format!("data:image/jpeg;base64,{}", b64))
        } else {
            // Fallback: try taking frame at 00:00:00
            let out2 = new_host_command("ffmpeg")
                .arg("-y")
                .arg("-ss").arg("00:00:00")
                .arg("-i").arg(&source_path)
                .arg("-vframes").arg("1")
                .arg("-s").arg("320x180")
                .arg("-f").arg("image2")
                .arg("-c:v").arg("mjpeg")
                .arg("-")
                .output()
                .map_err(|e| e.to_string())?;

            if out2.status.success() && !out2.stdout.is_empty() {
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&out2.stdout);
                Ok(format!("data:image/jpeg;base64,{}", b64))
            } else {
                Err("Failed to extract preview frame".to_string())
            }
        }
    })
    .await
    .map_err(|e| format!("Preview task error: {}", e))?
}

fn pick_video_paths_internal() -> Vec<String> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut paths_to_import: Vec<String> = Vec::new();

    // 1. Try kdialog (native KDE file dialog)
    if let Ok(out) = new_host_command("kdialog")
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
        if let Ok(out) = new_host_command("zenity")
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

    paths_to_import
}

fn pick_video_folder_internal() -> Option<String> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    // 1. Try kdialog (native KDE file dialog)
    if let Ok(out) = new_host_command("kdialog")
        .arg("--getexistingdirectory")
        .arg(&home)
        .arg("--title")
        .arg("Select Video Folder to Import")
        .output()
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let trimmed = stdout.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // 2. Try zenity fallback
    if let Ok(out) = new_host_command("zenity")
        .arg("--file-selection")
        .arg("--directory")
        .arg("--title=Select Video Folder to Import")
        .output()
    {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let trimmed = stdout.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

#[tauri::command]
fn select_video_files() -> Result<Vec<String>, String> {
    Ok(pick_video_paths_internal())
}

#[tauri::command]
fn select_video_folder() -> Result<Option<String>, String> {
    Ok(pick_video_folder_internal())
}

const SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "webm", "mkv", "mov", "gif", "avi", "flv", "m4v", "wmv", "ts"
];

fn is_supported_video_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        SUPPORTED_VIDEO_EXTENSIONS.iter().any(|&e| e == ext_lower)
    } else {
        false
    }
}

fn scan_sources_internal(sources: &[String]) -> Vec<String> {
    let mut resolved: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for src_str in sources {
        let path = Path::new(src_str);
        if !path.exists() {
            continue;
        }

        if path.is_file() {
            if is_supported_video_file(path) {
                if let Ok(canonical) = path.canonicalize() {
                    if seen.insert(canonical) {
                        resolved.push(path.to_string_lossy().to_string());
                    }
                } else if seen.insert(path.to_path_buf()) {
                    resolved.push(src_str.clone());
                }
            }
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.') && name != "node_modules"
                })
                .flatten()
            {
                let p = entry.path();
                if is_supported_video_file(p) {
                    if let Ok(canonical) = p.canonicalize() {
                        if seen.insert(canonical) {
                            resolved.push(p.to_string_lossy().to_string());
                        }
                    } else if seen.insert(p.to_path_buf()) {
                        resolved.push(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    resolved.sort();
    resolved
}

#[tauri::command]
fn scan_video_sources(sources: Vec<String>) -> Vec<String> {
    scan_sources_internal(&sources)
}

#[derive(serde::Serialize, Clone)]
pub struct BatchImportProgress {
    pub current: usize,
    pub total: usize,
    pub filename: String,
    pub percent: u8,
    pub status: String,
}

#[derive(serde::Serialize, Clone)]
pub struct BatchImportResult {
    pub imported: Vec<WallpaperItem>,
    pub total_requested: usize,
    pub failed: Vec<String>,
}

#[tauri::command]
async fn batch_import_wallpapers(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source_paths: Vec<String>,
) -> Result<BatchImportResult, String> {
    use tauri::{Emitter, Manager};

    let wp_mgr = state.wallpaper_mgr.clone();

    tokio::task::spawn_blocking(move || {
        let valid_paths = scan_sources_internal(&source_paths);
        let total = valid_paths.len();
        let mut imported = Vec::new();
        let mut failed = Vec::new();

        if total == 0 {
            return Ok(BatchImportResult {
                imported: Vec::new(),
                total_requested: 0,
                failed: Vec::new(),
            });
        }

        let mut library = wp_mgr.load_library();

        for (idx, path) in valid_paths.iter().enumerate() {
            let current = idx + 1;
            let filename = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("video")
                .to_string();
            let percent = ((current as f32 / total as f32) * 100.0).round() as u8;

            let progress = BatchImportProgress {
                current,
                total,
                filename: filename.clone(),
                percent,
                status: "processing".to_string(),
            };
            let _ = app.emit("batch-import-progress", &progress);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.emit("batch-import-progress", &progress);
            }

            match import_single_file_to_storage(&wp_mgr, path) {
                Ok(item) => {
                    library.push(item.clone());
                    imported.push(item);
                }
                Err(err) => {
                    eprintln!("[AnimatedLinux] Failed to import {}: {}", path, err);
                    failed.push(format!("{}: {}", filename, err));
                }
            }
        }

        if !imported.is_empty() {
            let _ = wp_mgr.save_library(&library);
        }

        let finish_progress = BatchImportProgress {
            current: total,
            total,
            filename: "Completed".to_string(),
            percent: 100,
            status: "finished".to_string(),
        };
        let _ = app.emit("batch-import-progress", &finish_progress);
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.emit("batch-import-progress", &finish_progress);
        }

        Ok(BatchImportResult {
            imported,
            total_requested: total,
            failed,
        })
    })
    .await
    .map_err(|e| format!("Batch import task failed: {}", e))?
}

#[tauri::command]
fn select_and_import_videos(state: State<'_, AppState>) -> Result<Vec<WallpaperItem>, String> {
    let paths_to_import = pick_video_paths_internal();
    let mut imported = Vec::new();
    for p in paths_to_import {
        if let Ok(item) = import_wallpaper_internal(&state.wallpaper_mgr, &p) {
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

        // Clean up any generated resolution variant files
        let storage_dir = state.wallpaper_mgr.get_storage_dir();
        for q in 1..=5 {
            let variant_path = storage_dir.join(format!("{}_q{}.mp4", item.id, q));
            let _ = std::fs::remove_file(variant_path);
        }

        state.wallpaper_mgr.save_library(&library)?;
    }

    let was_active = {
        let mut active_lock = state.wallpaper_mgr.active_wallpaper.lock().unwrap();
        if let Some(active) = &*active_lock {
            if active.id == id {
                *active_lock = None;
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    if was_active {
        let _ = state.wallpaper_mgr.stop();
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
async fn apply_wallpaper(state: State<'_, AppState>, wallpaper: WallpaperItem, screen: Option<i32>) -> Result<(), String> {
    state.request_manager.apply_wallpaper(wallpaper.clone(), screen).await?;
    state.steam_mgr.set_wallpaper_presence(&wallpaper.title);
    Ok(())
}

#[tauri::command]
async fn stop_wallpaper(state: State<'_, AppState>) -> Result<(), String> {
    state.request_manager.stop_wallpaper().await
}

#[tauri::command]
async fn reset_desktop_wallpaper(state: State<'_, AppState>) -> Result<(), String> {
    state.request_manager.reset_desktop().await
}

#[tauri::command]
fn get_active_wallpaper(state: State<'_, AppState>) -> Option<WallpaperItem> {
    state.wallpaper_mgr.active_wallpaper.lock().unwrap().clone()
}

#[tauri::command]
async fn set_playback_state(
    state: State<'_, AppState>,
    is_paused: bool,
    volume: u32,
    is_muted: bool,
    speed: f32,
) -> Result<(), String> {
    state.request_manager.set_playback_state(is_paused, volume, is_muted, speed).await
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
    inactive_black_screen: bool,
    restart_on_resume: bool,
    pause_per_monitor: bool,
    pause_detection_mode: Option<String>,
    main_display_id: Option<i32>,
) -> Result<PlaybackSettings, String> {
    state.wallpaper_mgr.update_performance_settings(
        quality_level,
        target_fps,
        pause_on_fullscreen,
        pause_policy,
        hardware_accel,
        inactive_black_screen,
        restart_on_resume,
        pause_per_monitor,
        pause_detection_mode,
        main_display_id,
    )
}

#[tauri::command]
fn set_main_display(state: State<'_, AppState>, display_id: i32) -> Result<PlaybackSettings, String> {
    state.wallpaper_mgr.set_main_display(display_id)
}

#[tauri::command]
fn get_caching_progress(state: State<'_, AppState>) -> Option<CachingProgressPayload> {
    state.caching_progress.lock().unwrap().clone()
}

#[tauri::command]
async fn set_video_geometry(
    state: State<'_, AppState>,
    fill_mode: u32,
    pan_x: f32,
    pan_y: f32,
) -> Result<(), String> {
    state.request_manager.set_geometry(fill_mode, pan_x, pan_y).await
}

#[tauri::command]
fn get_wallpaper_backend(state: State<'_, AppState>) -> String {
    state.wallpaper_mgr.get_active_backend_type()
}

#[tauri::command]
async fn set_wallpaper_backend(
    state: State<'_, AppState>,
    backend: String,
) -> Result<(), String> {
    state.request_manager.switch_backend(backend).await
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

    {
        let mut s = state.wallpaper_mgr.current_settings.lock().unwrap();
        s.quality_level = quality_level.clamp(1, 5);
        let updated_s = s.clone();
        drop(s);
        state.wallpaper_mgr.save_settings(&updated_s);
    }

    let active_id = {
        let active = state.wallpaper_mgr.active_wallpaper.lock().unwrap();
        active.as_ref().map(|w| w.id.clone())
    };
    if let Some(active_id) = active_id {
        if active_id == id {
            let req_mgr = state.request_manager.clone();
            let up = updated.clone();
            tauri::async_runtime::spawn(async move {
                let _ = req_mgr.apply_wallpaper(up, None).await;
            });
        }
    }

    Ok(updated)
}

#[derive(Debug, Clone)]
struct VariantEncodeResult {
    variant_path: String,
    resolution: Option<String>,
    duration_secs: Option<f64>,
    thumbnail_data: Option<String>,
    #[allow(dead_code)]
    already_cached: bool,
}

#[derive(Clone, serde::Serialize)]
pub struct CachingProgressPayload {
    pub current: usize,
    pub total: usize,
    pub id: String,
    pub title: String,
    pub percent: u32,
    pub status: String,
}

#[derive(Clone, serde::Serialize)]
pub struct CachingFinishedPayload {
    pub success_count: usize,
    pub total: usize,
    pub cancelled: bool,
}

static BATCH_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

fn encode_variant_internal(
    wallpapers_dir: PathBuf,
    item_id: String,
    item_format: String,
    current_file_path: String,
    quality_level: u32,
    target_fps: u32,
    thumbnail_path: String,
) -> Result<VariantEncodeResult, String> {
    let current_video_path = PathBuf::from(&current_file_path);
    if !current_video_path.exists() {
        return Err("Source video file does not exist".to_string());
    }

    let orig_backup = wallpapers_dir.join(format!("{}_source.{}", item_id, item_format));

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
        wallpapers_dir.join(format!("{}_q{}.mp4", item_id, quality_level))
    };

    if quality_level == 5 {
        return Ok(VariantEncodeResult {
            variant_path: target_variant_path.to_string_lossy().to_string(),
            resolution: None,
            duration_secs: None,
            thumbnail_data: None,
            already_cached: true,
        });
    }

    // Fast short-circuit: if variant already exists on disk and is non-empty, avoid redundant re-encoding
    if target_variant_path.exists() {
        if let Ok(meta) = target_variant_path.metadata() {
            if meta.len() > 0 {
                return Ok(VariantEncodeResult {
                    variant_path: target_variant_path.to_string_lossy().to_string(),
                    resolution: None,
                    duration_secs: None,
                    thumbnail_data: None,
                    already_cached: true,
                });
            }
        }
    }

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
            3 => 30,
            4 => 60,
            _ => 30,
        }
    };

    let temp_output = wallpapers_dir.join(format!("{}_temp_q{}.mp4", item_id, quality_level));
    let render_node = std::path::Path::new("/dev/dri/renderD128");

    let mut success = false;

    // Fast Hardware Accelerated Encoding via VA-API (Intel / AMD)
    if render_node.exists() {
        let vaapi_vf = format!("fps={},format=nv12,hwupload,scale_vaapi=w=-2:h={}", fps, max_height);
        let status = new_host_command("ffmpeg")
            .arg("-y")
            .arg("-vaapi_device").arg("/dev/dri/renderD128")
            .arg("-i").arg(&input_source)
            .arg("-vf").arg(&vaapi_vf)
            .arg("-c:v").arg("h264_vaapi")
            .arg("-qp").arg(crf)
            .arg("-an")
            .arg(&temp_output)
            .status();

        if let Ok(st) = status {
            if st.success() {
                success = true;
            }
        }
    }

    // CPU Fallback via libx264 with bounded threads to prevent UI and desktop compositor stutter
    if !success {
        let vf = format!("scale=-2:{},fps={},format=yuv420p", max_height, fps);
        let status = new_host_command("ffmpeg")
            .arg("-y")
            .arg("-i").arg(&input_source)
            .arg("-vf").arg(&vf)
            .arg("-c:v").arg("libx264")
            .arg("-pix_fmt").arg("yuv420p")
            .arg("-tune").arg("fastdecode")
            .arg("-preset").arg("veryfast")
            .arg("-threads").arg("2")
            .arg("-crf").arg(crf)
            .arg("-an")
            .arg(&temp_output)
            .status()
            .map_err(|e| format!("Failed to invoke ffmpeg: {}", e))?;

        if !status.success() {
            let _ = std::fs::remove_file(&temp_output);
            return Err("ffmpeg failed to compress video".to_string());
        }
    }

    let _ = std::fs::rename(&temp_output, &target_variant_path);

    let resolution = get_video_resolution(&target_variant_path);
    let duration_secs = get_video_duration(&target_variant_path);
    let thumb_path = PathBuf::from(&thumbnail_path);
    let thumbnail_data = if thumb_path.exists() {
        encode_thumbnail_base64(&thumb_path)
    } else {
        None
    };

    Ok(VariantEncodeResult {
        variant_path: target_variant_path.to_string_lossy().to_string(),
        resolution,
        duration_secs,
        thumbnail_data,
        already_cached: false,
    })
}

#[tauri::command]
async fn compress_wallpaper(
    state: State<'_, AppState>,
    id: String,
    quality_level: u32,
    target_fps: u32,
) -> Result<WallpaperItem, String> {
    let (item_id, item_format, item_fpath, item_tpath) = {
        let library = state.wallpaper_mgr.load_library();
        let item = library
            .iter()
            .find(|w| w.id == id)
            .ok_or_else(|| "Wallpaper not found".to_string())?;
        (item.id.clone(), item.format.clone(), item.file_path.clone(), item.thumbnail_path.clone())
    };

    let wallpapers_dir = state.wallpaper_mgr.get_storage_dir();

    let encode_res = tokio::task::spawn_blocking(move || {
        encode_variant_internal(
            wallpapers_dir,
            item_id,
            item_format,
            item_fpath,
            quality_level,
            target_fps,
            item_tpath,
        )
    })
    .await
    .map_err(|e| format!("Task spawn error: {}", e))??;

    let mut library = state.wallpaper_mgr.load_library();
    let pos = library
        .iter()
        .position(|w| w.id == id)
        .ok_or_else(|| "Wallpaper not found".to_string())?;

    let item = &mut library[pos];
    item.file_path = encode_res.variant_path;

    if !item.cached_qualities.contains(&quality_level) {
        item.cached_qualities.push(quality_level);
        item.cached_qualities.sort();
        item.cached_qualities.dedup();
    }

    if let Some(res) = encode_res.resolution {
        item.resolution = res;
    }
    if let Some(dur) = encode_res.duration_secs {
        item.duration_secs = dur;
    }
    if let Some(tdata) = encode_res.thumbnail_data {
        item.thumbnail_data = Some(tdata);
    }

    let updated = item.clone();
    state.wallpaper_mgr.save_library(&library)?;

    // If active, re-apply through RequestManager for thread-safe actor serialization
    let active_id = {
        let active = state.wallpaper_mgr.active_wallpaper.lock().unwrap();
        active.as_ref().map(|w| w.id.clone())
    };
    if let Some(active_id) = active_id {
        if active_id == id {
            let req_mgr = state.request_manager.clone();
            let up = updated.clone();
            tauri::async_runtime::spawn(async move {
                let _ = req_mgr.apply_wallpaper(up, None).await;
            });
        }
    }

    Ok(updated)
}

#[tauri::command]
async fn compress_all_wallpapers(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    quality_level: u32,
    target_fps: u32,
) -> Result<CachingFinishedPayload, String> {
    use tauri::Emitter;
    use tauri::Manager;

    BATCH_CANCEL_REQUESTED.store(false, Ordering::SeqCst);

    if quality_level == 5 {
        let payload = CachingFinishedPayload {
            success_count: 0,
            total: 0,
            cancelled: false,
        };
        let _ = app.emit("caching-finished", &payload);
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.emit("caching-finished", &payload);
        }
        return Ok(payload);
    }

    let library = state.wallpaper_mgr.load_library();
    let wallpapers_dir = state.wallpaper_mgr.get_storage_dir();

    // Identify wallpapers that need encoding or cache recognition
    let mut to_process: Vec<WallpaperItem> = Vec::new();
    for item in &library {
        let variant_path = wallpapers_dir.join(format!("{}_q{}.mp4", item.id, quality_level));
        let exists = variant_path.exists() && variant_path.metadata().map(|m| m.len() > 0).unwrap_or(false);
        if !exists || !item.cached_qualities.contains(&quality_level) {
            to_process.push(item.clone());
        }
    }

    let total = to_process.len();
    if total == 0 {
        let payload = CachingFinishedPayload {
            success_count: 0,
            total: 0,
            cancelled: false,
        };
        let _ = app.emit("caching-finished", &payload);
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.emit("caching-finished", &payload);
        }
        return Ok(payload);
    }

    // Use bounded concurrency of 2 parallel transcode workers to keep desktop responsive
    let semaphore = Arc::new(tokio::sync::Semaphore::new(2));
    let finished_counter = Arc::new(AtomicUsize::new(0));
    let success_counter = Arc::new(AtomicUsize::new(0));
    let caching_progress_state = state.caching_progress.clone();

    // Set initial 0 / total progress
    {
        let init_payload = CachingProgressPayload {
            current: 0,
            total,
            id: String::new(),
            title: String::new(),
            percent: 0,
            status: "Starting parallel variant caching...".to_string(),
        };
        *caching_progress_state.lock().unwrap() = Some(init_payload.clone());
        let _ = app.emit("caching-progress", &init_payload);
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.emit("caching-progress", &init_payload);
        }
    }

    let mut handles = Vec::new();

    for item in to_process {
        let sem = semaphore.clone();
        let app_handle = app.clone();
        let wp_dir = wallpapers_dir.clone();
        let finished = finished_counter.clone();
        let success = success_counter.clone();
        let item_clone = item.clone();
        let progress_state = caching_progress_state.clone();

        let handle = tokio::spawn(async move {
            if BATCH_CANCEL_REQUESTED.load(Ordering::SeqCst) {
                return None;
            }

            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return None,
            };

            if BATCH_CANCEL_REQUESTED.load(Ordering::SeqCst) {
                return None;
            }

            let cur_finished = finished.load(Ordering::SeqCst);
            let percent = ((cur_finished as f32 / total as f32) * 100.0) as u32;

            let start_payload = CachingProgressPayload {
                current: cur_finished,
                total,
                id: item_clone.id.clone(),
                title: item_clone.title.clone(),
                percent,
                status: format!("Encoding \"{}\"...", item_clone.title),
            };
            *progress_state.lock().unwrap() = Some(start_payload.clone());
            let _ = app_handle.emit("caching-progress", &start_payload);
            if let Some(w) = app_handle.get_webview_window("main") {
                let _ = w.emit("caching-progress", &start_payload);
            }

            let res = tokio::task::spawn_blocking({
                let id = item_clone.id.clone();
                let fmt = item_clone.format.clone();
                let fpath = item_clone.file_path.clone();
                let tpath = item_clone.thumbnail_path.clone();
                move || {
                    encode_variant_internal(
                        wp_dir,
                        id,
                        fmt,
                        fpath,
                        quality_level,
                        target_fps,
                        tpath,
                    )
                }
            }).await;

            let new_finished = finished.fetch_add(1, Ordering::SeqCst) + 1;
            let new_percent = ((new_finished as f32 / total as f32) * 100.0) as u32;

            let item_res = match res {
                Ok(Ok(encode_res)) => {
                    success.fetch_add(1, Ordering::SeqCst);
                    let _ = app_handle.emit("wallpaper-cached", serde_json::json!({
                        "id": item_clone.id,
                        "quality_level": quality_level
                    }));
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.emit("wallpaper-cached", serde_json::json!({
                            "id": item_clone.id,
                            "quality_level": quality_level
                        }));
                    }
                    Some((item_clone.id.clone(), encode_res))
                }
                _ => None,
            };

            let finish_payload = CachingProgressPayload {
                current: new_finished,
                total,
                id: item_clone.id.clone(),
                title: item_clone.title.clone(),
                percent: new_percent,
                status: if new_finished == total {
                    "All items completed".to_string()
                } else {
                    format!("Completed \"{}\"", item_clone.title)
                },
            };
            *progress_state.lock().unwrap() = Some(finish_payload.clone());
            let _ = app_handle.emit("caching-progress", &finish_payload);
            if let Some(w) = app_handle.get_webview_window("main") {
                let _ = w.emit("caching-progress", &finish_payload);
            }

            item_res
        });

        handles.push(handle);
    }

    let mut encoded_results = Vec::new();
    for h in handles {
        if let Ok(Some(res)) = h.await {
            encoded_results.push(res);
        }
    }

    let cancelled = BATCH_CANCEL_REQUESTED.load(Ordering::SeqCst);
    let success_count = success_counter.load(Ordering::SeqCst);

    // Batch update library
    if !encoded_results.is_empty() {
        let mut lib = state.wallpaper_mgr.load_library();
        for (id, encode_res) in encoded_results {
            if let Some(it) = lib.iter_mut().find(|w| w.id == id) {
                if !it.cached_qualities.contains(&quality_level) {
                    it.cached_qualities.push(quality_level);
                    it.cached_qualities.sort();
                    it.cached_qualities.dedup();
                }
                if let Some(res) = encode_res.resolution {
                    it.resolution = res;
                }
                if let Some(dur) = encode_res.duration_secs {
                    it.duration_secs = dur;
                }
                if let Some(tdata) = encode_res.thumbnail_data {
                    it.thumbnail_data = Some(tdata);
                }
            }
        }
        let _ = state.wallpaper_mgr.save_library(&lib);
    }

    *caching_progress_state.lock().unwrap() = None;

    let payload = CachingFinishedPayload {
        success_count,
        total,
        cancelled,
    };
    let _ = app.emit("caching-finished", &payload);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.emit("caching-finished", &payload);
    }

    Ok(payload)
}

#[tauri::command]
fn cancel_batch_compression() -> Result<(), String> {
    BATCH_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    Ok(())
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
    let out = new_host_command("ffprobe")
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
    let out = new_host_command("ffprobe")
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

#[derive(Debug, serde::Serialize)]
pub struct CacheStats {
    pub cached_variants_count: usize,
    pub total_cache_bytes: u64,
}

#[tauri::command]
fn open_storage_dir(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.wallpaper_mgr.get_storage_dir();
    new_host_command("xdg-open")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("Failed to open directory with xdg-open: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    let dir = state.wallpaper_mgr.get_storage_dir();
    tokio::task::spawn_blocking(move || {
        let mut count = 0;
        let mut bytes = 0;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    if fname.contains("_q") && fname.ends_with(".mp4") {
                        count += 1;
                        if let Ok(meta) = entry.metadata() {
                            bytes += meta.len();
                        }
                    }
                }
            }
        }
        Ok(CacheStats {
            cached_variants_count: count,
            total_cache_bytes: bytes,
        })
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
async fn clear_variant_cache(state: State<'_, AppState>) -> Result<usize, String> {
    let dir = state.wallpaper_mgr.get_storage_dir();
    let deleted = tokio::task::spawn_blocking(move || {
        let mut del_count = 0;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    if fname.contains("_q") && fname.ends_with(".mp4") {
                        if std::fs::remove_file(&path).is_ok() {
                            del_count += 1;
                        }
                    }
                }
            }
        }
        del_count
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?;

    let mut library = state.wallpaper_mgr.load_library();
    for item in &mut library {
        item.cached_qualities = vec![5];
    }
    state.wallpaper_mgr.save_library(&library)?;

    let mut active_lock = state.wallpaper_mgr.active_wallpaper.lock().unwrap();
    if let Some(active) = &mut *active_lock {
        active.cached_qualities = vec![5];
    }

    Ok(deleted)
}

#[tauri::command]
fn minimize_window(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn toggle_maximize_window(window: tauri::WebviewWindow) {
    if let Ok(is_maximized) = window.is_maximized() {
        if is_maximized {
            let _ = window.unmaximize();
        } else {
            let _ = window.maximize();
        }
    }
}

#[tauri::command]
fn is_window_maximized(window: tauri::WebviewWindow) -> bool {
    window.is_maximized().unwrap_or(false)
}

static SHOULD_EXIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

fn resolve_app_executable_and_icon() -> (String, String) {
    let mut exe_str = String::new();
    let mut icon_str = String::new();

    // 1. Prioritize launch.sh so desktop entries and shortcuts execute through the launch script
    if let Ok(cur) = std::env::current_exe() {
        let mut p = cur.parent();
        while let Some(dir) = p {
            let launch_root = dir.join("launch.sh");
            let launch_setup = dir.join("setup").join("launch.sh");
            if launch_root.is_file() {
                exe_str = launch_root.to_string_lossy().to_string();
                break;
            } else if launch_setup.is_file() {
                exe_str = launch_setup.to_string_lossy().to_string();
                break;
            }
            p = dir.parent();
        }
    }

    if exe_str.is_empty() {
        if let Ok(cwd) = std::env::current_dir() {
            let launch_root = cwd.join("launch.sh");
            let launch_setup = cwd.join("setup").join("launch.sh");
            if launch_root.is_file() {
                exe_str = launch_root.to_string_lossy().to_string();
            } else if launch_setup.is_file() {
                exe_str = launch_setup.to_string_lossy().to_string();
            }
        }
    }

    // 2. Fallback to native binary if launch script not located
    if exe_str.is_empty() {
        if let Ok(cur) = std::env::current_exe() {
            let cur_str = cur.to_string_lossy().to_string();
            if !cur_str.contains("/.mount_") {
                exe_str = cur_str;
            } else {
                exe_str = "animated-linux".to_string();
            }
        } else {
            exe_str = "animated-linux".to_string();
        }
    }

    // 3. Resolve icon
    if let Ok(current_exe) = std::env::current_exe() {
        let mut check_dir = current_exe.parent();
        while let Some(dir) = check_dir {
            let icon_file = dir.join("src-tauri").join("icons").join("128x128.png");
            let alt_icon = dir.join("src-tauri").join("icons").join("icon.png");

            if icon_file.exists() {
                icon_str = icon_file.to_string_lossy().to_string();
                break;
            } else if alt_icon.exists() {
                icon_str = alt_icon.to_string_lossy().to_string();
                break;
            }
            check_dir = dir.parent();
        }
    }

    if icon_str.is_empty() {
        if let Ok(cwd) = std::env::current_dir() {
            let icon_file = cwd.join("src-tauri").join("icons").join("128x128.png");
            let alt_icon = cwd.join("src-tauri").join("icons").join("icon.png");
            if icon_file.exists() {
                icon_str = icon_file.to_string_lossy().to_string();
            } else if alt_icon.exists() {
                icon_str = alt_icon.to_string_lossy().to_string();
            }
        }
    }

    if icon_str.is_empty() {
        icon_str = "animated-linux".to_string();
    }

    (exe_str, icon_str)
}

fn install_system_icon(source_icon: &str) {
    if let Some(data_dir) = dirs::data_local_dir() {
        let icon_dest_dir = data_dir.join("icons").join("hicolor").join("128x128").join("apps");
        let _ = std::fs::create_dir_all(&icon_dest_dir);
        let dest_icon = icon_dest_dir.join("animated-linux.png");
        let src = Path::new(source_icon);
        if src.exists() {
            let _ = std::fs::copy(src, &dest_icon);
        } else if !dest_icon.exists() {
            let embedded_bytes = include_bytes!("../icons/128x128.png");
            let _ = std::fs::write(&dest_icon, embedded_bytes);
        }
    }
}

#[tauri::command]
fn get_app_menu_status() -> bool {
    if let Some(data_dir) = dirs::data_local_dir() {
        let desktop_file = data_dir.join("applications").join("animated-linux.desktop");
        desktop_file.exists()
    } else {
        false
    }
}

#[tauri::command]
fn set_app_menu_entry(enable: bool) -> Result<bool, String> {
    let data_dir = dirs::data_local_dir().ok_or_else(|| "Could not locate user data directory".to_string())?;
    let apps_dir = data_dir.join("applications");
    let desktop_file = apps_dir.join("animated-linux.desktop");

    if enable {
        std::fs::create_dir_all(&apps_dir).map_err(|e| format!("Failed to create applications directory: {}", e))?;
        let (exe_path, icon_source) = resolve_app_executable_and_icon();
        install_system_icon(&icon_source);

        let desktop_content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Animated Linux\n\
             GenericName=Animated Wallpaper Engine\n\
             Comment=Hardware-accelerated animated wallpaper engine for Linux\n\
             Exec=\"{}\"\n\
             Icon=animated-linux\n\
             Terminal=false\n\
             Categories=Utility;Graphics;Settings;\n\
             Keywords=wallpaper;video;animated;engine;plasma;kde;\n\
             StartupWMClass=animated-linux\n",
            exe_path
        );

        std::fs::write(&desktop_file, desktop_content).map_err(|e| format!("Failed to write application menu desktop entry: {}", e))?;
        let _ = new_host_command("update-desktop-database").arg(&apps_dir).output();
        let _ = new_host_command("kbuildsycoca6").output();
        Ok(true)
    } else {
        if desktop_file.exists() {
            std::fs::remove_file(&desktop_file).map_err(|e| format!("Failed to remove application menu desktop entry: {}", e))?;
        }
        let _ = new_host_command("update-desktop-database").arg(&apps_dir).output();
        let _ = new_host_command("kbuildsycoca6").output();
        Ok(false)
    }
}

#[tauri::command]
fn get_autostart_status() -> bool {
    if let Some(config_dir) = dirs::config_dir() {
        let desktop_file = config_dir.join("autostart").join("animated-linux.desktop");
        desktop_file.exists()
    } else {
        false
    }
}

#[tauri::command]
fn set_autostart(enable: bool) -> Result<bool, String> {
    let config_dir = dirs::config_dir().ok_or_else(|| "Could not locate user config directory".to_string())?;
    let autostart_dir = config_dir.join("autostart");
    let desktop_file = autostart_dir.join("animated-linux.desktop");

    if enable {
        std::fs::create_dir_all(&autostart_dir).map_err(|e| format!("Failed to create autostart directory: {}", e))?;
        let (exe_path, icon_source) = resolve_app_executable_and_icon();
        install_system_icon(&icon_source);

        let desktop_content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Animated Linux\n\
             GenericName=Animated Wallpaper Engine\n\
             Comment=Hardware-accelerated animated wallpaper engine for Linux\n\
             Exec=\"{}\" --autostart\n\
             Icon=animated-linux\n\
             Terminal=false\n\
             Categories=Utility;Settings;\n\
             X-GNOME-Autostart-enabled=true\n",
            exe_path
        );

        std::fs::write(&desktop_file, desktop_content).map_err(|e| format!("Failed to write autostart desktop entry: {}", e))?;
        Ok(true)
    } else {
        if desktop_file.exists() {
            std::fs::remove_file(&desktop_file).map_err(|e| format!("Failed to remove autostart desktop entry: {}", e))?;
        }
        Ok(false)
    }
}

#[tauri::command]
fn get_start_minimized(state: State<'_, AppState>) -> bool {
    state.wallpaper_mgr.get_start_minimized()
}

#[tauri::command]
fn set_start_minimized(state: State<'_, AppState>, enable: bool) -> Result<bool, String> {
    state.wallpaper_mgr.set_start_minimized(enable)
}

#[tauri::command]
async fn safe_exit_app(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    SHOULD_EXIT.store(true, std::sync::atomic::Ordering::SeqCst);
    state.request_manager.safe_quit(app).await
}

#[tauri::command]
async fn exit_app(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    SHOULD_EXIT.store(true, std::sync::atomic::Ordering::SeqCst);
    state.request_manager.safe_quit(app).await
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("GTK_THEME", "Adwaita:dark");
        std::env::set_var("GTK_APPLICATION_PREFER_DARK_THEME", "1");
        // Prevent WebKitGTK explicit sync crashes on Wayland (Error 71 dispatching to Wayland display)
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    let steam_mgr = SteamManager::new(480);
    let wallpaper_mgr = Arc::new(WallpaperManager::new());
    let workshop_mgr = WorkshopManager::new();
    let request_manager = Arc::new(RequestManager::new(wallpaper_mgr.clone()));

    let caching_progress = Arc::new(std::sync::Mutex::new(None));

    let app_state = AppState {
        wallpaper_mgr: wallpaper_mgr.clone(),
        steam_mgr: steam_mgr.clone(),
        workshop_mgr,
        request_manager: request_manager.clone(),
        caching_progress,
    };

    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if !SHOULD_EXIT.load(std::sync::atomic::Ordering::SeqCst) {
                    api.prevent_close();
                    use tauri::Emitter;
                    let _ = window.emit("window-close-requested", ());
                }
            }
        })
        .setup(move |app| {
            let args: Vec<String> = std::env::args().collect();
            let is_explicit_minimized = args.iter().any(|a| a == "--minimized" || a == "--hidden");
            let is_explicit_show = args.iter().any(|a| a == "--open" || a == "--show");
            let is_autostart = args.iter().any(|a| a == "--autostart");
            let start_minimized = wallpaper_mgr.get_start_minimized();

            let should_hide = !is_explicit_show && (is_explicit_minimized || start_minimized || (is_autostart && start_minimized));
            if should_hide {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("[Tray] Warning: could not initialize system tray: {}", e);
            }

            // Automatically restore active wallpaper and state on app launch if present upon exiting the app
            let req_mgr_init = request_manager.clone();
            let wp_mgr_init = wallpaper_mgr.clone();
            tauri::async_runtime::spawn(async move {
                // Short grace interval allowing window/display/desktop session to stabilize
                tokio::time::sleep(std::time::Duration::from_millis(350)).await;
                let maybe_active = wp_mgr_init.active_wallpaper.lock().unwrap().clone();
                if let Some(active) = maybe_active {
                    println!("[AnimatedLinux] Automatically loading last active wallpaper state on app launch: {}", active.title);
                    if let Err(e) = req_mgr_init.apply_wallpaper(active.clone(), None).await {
                        eprintln!("[AnimatedLinux] Warning on initial wallpaper apply: {}, retrying once...", e);
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        let retry_active = wp_mgr_init.active_wallpaper.lock().unwrap().clone();
                        if let Some(act) = retry_active {
                            let _ = req_mgr_init.apply_wallpaper(act, None).await;
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
            get_quick_preview,
            select_video_files,
            select_video_folder,
            scan_video_sources,
            batch_import_wallpapers,
            select_and_import_videos,
            delete_wallpaper,
            rename_wallpaper,
            apply_wallpaper,
            stop_wallpaper,
            reset_desktop_wallpaper,
            get_active_wallpaper,
            set_playback_state,
            get_performance_settings,
            update_performance_settings,
            get_caching_progress,
            compress_wallpaper,
            compress_all_wallpapers,
            cancel_batch_compression,
            switch_wallpaper_quality,
            set_video_geometry,
            get_monitors,
            get_steam_status,
            get_workshop_items,
            add_to_steam_client,
            minimize_window,
            toggle_maximize_window,
            is_window_maximized,
            hide_window,
            exit_app,
            safe_exit_app,
            open_storage_dir,
            get_cache_stats,
            clear_variant_cache,
            get_autostart_status,
            set_autostart,
            get_start_minimized,
            set_start_minimized,
            get_app_menu_status,
            set_app_menu_entry,
            get_wallpaper_backend,
            set_wallpaper_backend,
            set_main_display
        ])
        .run(tauri::generate_context!())
        .expect("error while running AetherPaper tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_app_executable_and_icon() {
        let (exe, icon) = resolve_app_executable_and_icon();
        println!("Resolved exe: {}", exe);
        println!("Resolved icon: {}", icon);
        assert!(!exe.is_empty());
        assert!(!icon.is_empty());
    }

    #[test]
    fn test_app_menu_entry_generation() {
        let (expected_exe, icon) = resolve_app_executable_and_icon();
        assert!(!expected_exe.is_empty());
        assert!(!icon.is_empty());

        match set_app_menu_entry(true) {
            Ok(_) => {
                if let Some(data_dir) = dirs::data_local_dir() {
                    let desktop_file = data_dir.join("applications").join("animated-linux.desktop");
                    if desktop_file.exists() {
                        let content = std::fs::read_to_string(&desktop_file).unwrap();
                        assert!(content.contains(&format!("Exec=\"{}\"", expected_exe)));
                        assert!(content.contains("Icon=animated-linux"));
                    }
                }
            }
            Err(e) => {
                println!("Skipping desktop file write test in restricted environment: {}", e);
            }
        }
    }

    #[test]
    fn test_scan_sources_internal_recursive() {
        let temp_dir = std::env::temp_dir().join(format!("anim_linux_test_{}", uuid_simple()));
        let sub_dir = temp_dir.join("nested_folder");
        let hidden_dir = temp_dir.join(".hidden");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::create_dir_all(&hidden_dir).unwrap();

        let vid1 = temp_dir.join("sample.mp4");
        let vid2 = sub_dir.join("clip.WEBM");
        let txt = temp_dir.join("document.txt");
        let hidden_vid = hidden_dir.join("secret.mp4");

        std::fs::write(&vid1, b"mock video 1").unwrap();
        std::fs::write(&vid2, b"mock video 2").unwrap();
        std::fs::write(&txt, b"mock text").unwrap();
        std::fs::write(&hidden_vid, b"mock hidden").unwrap();

        let sources = vec![temp_dir.to_string_lossy().to_string()];
        let found = scan_sources_internal(&sources);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|f| f.ends_with("sample.mp4")));
        assert!(found.iter().any(|f| f.ends_with("clip.WEBM")));
        assert!(!found.iter().any(|f| f.ends_with("document.txt")));
        assert!(!found.iter().any(|f| f.ends_with("secret.mp4")));
    }
}
