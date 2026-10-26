use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use crate::AppState;

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItem::with_id(app, "open_window", "Open Animated Linux", true, None::<&str>)?;
    let play_pause_item = MenuItem::with_id(app, "toggle_pause", "Pause / Resume Wallpaper", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop_wallpaper", "Stop Wallpaper", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Animated Linux", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_item, &play_pause_item, &stop_item, &quit_item])?;

    let tray_builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Animated Linux")
        .show_menu_on_left_click(false);

    let icon = app.default_window_icon()
        .cloned()
        .unwrap_or_else(|| tauri::include_image!("icons/icon.png"));

    let _tray = tray_builder
        .icon(icon)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "open_window" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
                "toggle_pause" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let is_paused = state.wallpaper_mgr.is_paused();
                        let target_paused = !is_paused;
                        let (volume, is_muted, speed) = {
                            let s = state.wallpaper_mgr.get_settings();
                            (s.volume, s.is_muted, s.speed)
                        };
                        let req_mgr = state.request_manager.clone();
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if req_mgr.set_playback_state(target_paused, volume, is_muted, speed).await.is_ok() {
                                use tauri::Emitter;
                                let _ = app_handle.emit("wallpaper-playback-changed", serde_json::json!({
                                    "is_paused": target_paused
                                }));
                            }
                        });
                    }
                }
                "stop_wallpaper" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let req_mgr = state.request_manager.clone();
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if req_mgr.stop_wallpaper().await.is_ok() {
                                use tauri::Emitter;
                                let _ = app_handle.emit("wallpaper-stopped", ());
                            }
                        });
                    }
                }
                "quit" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        state.request_manager.send_safe_quit_detached(app.clone());
                    } else {
                        app.exit(0);
                    }
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let is_visible = window.is_visible().unwrap_or(false);
                    if is_visible {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
