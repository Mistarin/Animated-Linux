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
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct AppState {
    pub wallpaper_mgr: Arc<WallpaperManager>,
    pub steam_mgr: Arc<SteamManager>,
    pub workshop_mgr: WorkshopManager,
    pub request_manager: Arc<RequestManager>,
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

        let temp_output = wallpapers_dir.join(format!("{}_temp_q{}.mp4", id, quality_level));
        let render_node = std::path::Path::new("/dev/dri/renderD128");

        let mut success = false;

        // Fast Hardware Accelerated Encoding via VA-API (Intel/AMD)
        if render_node.exists() {
            let vaapi_vf = format!("fps={},format=nv12,hwupload,scale_vaapi=w=-2:h={}", fps, max_height);
            let status = Command::new("ffmpeg")
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

        // CPU Fallback via libx264
        if !success {
            let vf = format!("scale=-2:{},fps={}", max_height, fps);
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

#[derive(Debug, serde::Serialize)]
pub struct CacheStats {
    pub cached_variants_count: usize,
    pub total_cache_bytes: u64,
}

#[tauri::command]
fn open_storage_dir(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.wallpaper_mgr.get_storage_dir();
    Command::new("xdg-open")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("Failed to open directory with xdg-open: {}", e))?;
    Ok(())
}

#[tauri::command]
fn get_cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    let dir = state.wallpaper_mgr.get_storage_dir();
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
}

#[tauri::command]
fn clear_variant_cache(state: State<'_, AppState>) -> Result<usize, String> {
    let dir = state.wallpaper_mgr.get_storage_dir();
    let mut deleted = 0;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                if fname.contains("_q") && fname.ends_with(".mp4") {
                    if std::fs::remove_file(&path).is_ok() {
                        deleted += 1;
                    }
                }
            }
        }
    }

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
        let current_exe = std::env::current_exe().map_err(|e| format!("Failed to determine current executable path: {}", e))?;
        let exe_str = current_exe.to_string_lossy();

        let icon_path = current_exe.parent()
            .map(|p| p.join("icons").join("icon.png"))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "animated-linux".to_string());

        let desktop_content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Animated Linux\n\
             Comment=Hardware-accelerated animated wallpaper engine for Linux\n\
             Exec=\"{}\" --autostart\n\
             Icon={}\n\
             Terminal=false\n\
             Categories=Utility;Settings;\n\
             X-GNOME-Autostart-enabled=true\n",
            exe_str, icon_path
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
    }

    let steam_mgr = SteamManager::new(480);
    let wallpaper_mgr = Arc::new(WallpaperManager::new());
    let workshop_mgr = WorkshopManager::new();
    let request_manager = Arc::new(RequestManager::new(wallpaper_mgr.clone()));

    let app_state = AppState {
        wallpaper_mgr: wallpaper_mgr.clone(),
        steam_mgr: steam_mgr.clone(),
        workshop_mgr,
        request_manager: request_manager.clone(),
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
            let is_autostart = args.iter().any(|a| a == "--autostart" || a == "--minimized" || a == "--hidden");
            if is_autostart {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("[Tray] Warning: could not initialize system tray: {}", e);
            }

            // Restore active wallpaper playback if present via RequestManager
            let req_mgr_init = request_manager.clone();
            let wp_mgr_init = wallpaper_mgr.clone();
            tauri::async_runtime::spawn(async move {
                let maybe_active = wp_mgr_init.active_wallpaper.lock().unwrap().clone();
                if let Some(active) = maybe_active {
                    let _ = req_mgr_init.apply_wallpaper(active, None).await;
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
            get_active_wallpaper,
            set_playback_state,
            get_performance_settings,
            update_performance_settings,
            compress_wallpaper,
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
            get_wallpaper_backend,
            set_wallpaper_backend
        ])
        .run(tauri::generate_context!())
        .expect("error while running AetherPaper tauri application");
}
