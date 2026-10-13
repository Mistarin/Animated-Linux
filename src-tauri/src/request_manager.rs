// Copyright (C) 2026 Animated Linux Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::wallpaper::backend::WallpaperItem;
use crate::wallpaper::WallpaperManager;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot};

/// Represents an asynchronous action request in the application queue.
pub enum ActionRequest {
    ApplyWallpaper {
        wallpaper: WallpaperItem,
        screen: Option<i32>,
        responder: oneshot::Sender<Result<(), String>>,
    },
    StopWallpaper {
        responder: oneshot::Sender<Result<(), String>>,
    },
    SetPlaybackState {
        is_paused: bool,
        volume: u32,
        is_muted: bool,
        speed: f32,
        responder: oneshot::Sender<Result<(), String>>,
    },
    SetGeometry {
        fill_mode: u32,
        pan_x: f32,
        pan_y: f32,
        responder: oneshot::Sender<Result<(), String>>,
    },
    SafeQuit {
        app_handle: AppHandle,
        responder: oneshot::Sender<Result<(), String>>,
    },
}

/// Central Request Manager that sequences all D-Bus and wallpaper backend actions
/// through a FIFO actor queue to prevent race conditions and guarantee safe shutdowns.
#[derive(Clone)]
pub struct RequestManager {
    sender: mpsc::Sender<ActionRequest>,
}

impl RequestManager {
    pub fn new(wallpaper_mgr: Arc<WallpaperManager>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<ActionRequest>(128);

        tauri::async_runtime::spawn(async move {
            println!("[AnimatedLinux] RequestManager action worker loop initialized");
            while let Some(request) = receiver.recv().await {
                match request {
                    ActionRequest::ApplyWallpaper { wallpaper, screen, responder } => {
                        let res = wallpaper_mgr.apply_wallpaper(wallpaper, screen);
                        let _ = responder.send(res);
                    }
                    ActionRequest::StopWallpaper { responder } => {
                        let res = wallpaper_mgr.stop();
                        let _ = responder.send(res);
                    }
                    ActionRequest::SetPlaybackState { is_paused, volume, is_muted, speed, responder } => {
                        let mut res = wallpaper_mgr.set_paused(is_paused);
                        if res.is_ok() {
                            res = wallpaper_mgr.set_volume(volume, is_muted);
                        }
                        if res.is_ok() {
                            res = wallpaper_mgr.set_speed(speed);
                        }
                        let _ = responder.send(res);
                    }
                    ActionRequest::SetGeometry { fill_mode, pan_x, pan_y, responder } => {
                        let res = wallpaper_mgr.set_geometry(fill_mode, pan_x, pan_y);
                        let _ = responder.send(res);
                    }
                    ActionRequest::SafeQuit { app_handle, responder } => {
                        println!("[AnimatedLinux] Executing safe quit sequence: stopping wallpaper backend and restoring static desktop...");
                        let stop_res = wallpaper_mgr.stop();
                        if let Err(ref e) = stop_res {
                            eprintln!("[AnimatedLinux] Warning during stop on quit: {}", e);
                        }
                        let _ = responder.send(stop_res);

                        println!("[AnimatedLinux] Desktop background restored. Exiting application gracefully.");
                        app_handle.exit(0);
                        break;
                    }
                }
            }
        });

        Self { sender }
    }

    pub async fn apply_wallpaper(&self, wallpaper: WallpaperItem, screen: Option<i32>) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.sender
            .send(ActionRequest::ApplyWallpaper { wallpaper, screen, responder })
            .await
            .map_err(|e| format!("Failed to enqueue ApplyWallpaper request: {}", e))?;
        receiver.await.map_err(|e| format!("Failed to receive ApplyWallpaper response: {}", e))?
    }

    pub async fn stop_wallpaper(&self) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.sender
            .send(ActionRequest::StopWallpaper { responder })
            .await
            .map_err(|e| format!("Failed to enqueue StopWallpaper request: {}", e))?;
        receiver.await.map_err(|e| format!("Failed to receive StopWallpaper response: {}", e))?
    }

    pub async fn set_playback_state(&self, is_paused: bool, volume: u32, is_muted: bool, speed: f32) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.sender
            .send(ActionRequest::SetPlaybackState { is_paused, volume, is_muted, speed, responder })
            .await
            .map_err(|e| format!("Failed to enqueue SetPlaybackState request: {}", e))?;
        receiver.await.map_err(|e| format!("Failed to receive SetPlaybackState response: {}", e))?
    }

    pub async fn set_geometry(&self, fill_mode: u32, pan_x: f32, pan_y: f32) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.sender
            .send(ActionRequest::SetGeometry { fill_mode, pan_x, pan_y, responder })
            .await
            .map_err(|e| format!("Failed to enqueue SetGeometry request: {}", e))?;
        receiver.await.map_err(|e| format!("Failed to receive SetGeometry response: {}", e))?
    }

    pub async fn safe_quit(&self, app_handle: AppHandle) -> Result<(), String> {
        let (responder, receiver) = oneshot::channel();
        self.sender
            .send(ActionRequest::SafeQuit { app_handle, responder })
            .await
            .map_err(|e| format!("Failed to enqueue SafeQuit request: {}", e))?;
        receiver.await.map_err(|e| format!("Failed to receive SafeQuit response: {}", e))?
    }

    pub fn send_safe_quit_detached(&self, app_handle: AppHandle) {
        let sender = self.sender.clone();
        tauri::async_runtime::spawn(async move {
            let (responder, receiver) = oneshot::channel();
            if let Ok(()) = sender.send(ActionRequest::SafeQuit { app_handle, responder }).await {
                let _ = receiver.await;
            }
        });
    }
}
