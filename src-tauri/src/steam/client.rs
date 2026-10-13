use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamStatus {
    pub is_connected: bool,
    pub user_name: String,
    pub steam_id: String,
    pub app_id: u32,
}

pub struct SteamManager {
    #[cfg(feature = "steam")]
    client: Option<steamworks::Client>,
    pub status: Mutex<SteamStatus>,
}

#[cfg(feature = "steam")]
fn ensure_appid_file(path: &str, app_id: u32) {
    let id_str = app_id.to_string();
    if let Ok(existing) = std::fs::read_to_string(path) {
        if existing.trim() == id_str {
            return;
        }
    }
    let _ = std::fs::write(path, id_str);
}

impl SteamManager {
    #[cfg(feature = "steam")]
    pub fn new(app_id: u32) -> Arc<Self> {
        ensure_appid_file("steam_appid.txt", app_id);
        ensure_appid_file("src-tauri/steam_appid.txt", app_id);

        let (client_opt, status) = match steamworks::Client::init_app(app_id) {
            Ok(client) => {
                let user_name = client.friends().name();
                let steam_id = client.user().steam_id().raw().to_string();
                let status = SteamStatus {
                    is_connected: true,
                    user_name,
                    steam_id,
                    app_id,
                };
                (Some(client), status)
            }
            Err(e) => {
                eprintln!("[Steam] Steamworks init notice (offline / standalone mode): {}", e);
                let status = SteamStatus {
                    is_connected: false,
                    user_name: "Desktop Standalone Mode".to_string(),
                    steam_id: "0".to_string(),
                    app_id,
                };
                (None, status)
            }
        };

        let manager = Arc::new(Self {
            client: client_opt,
            status: Mutex::new(status),
        });

        if let Some(ref client) = manager.client {
            client.friends().set_rich_presence("status", Some("Active: Animated Linux Engine"));
            client.friends().set_rich_presence("steam_display", Some("#StatusActive"));
        }

        manager
    }

    #[cfg(not(feature = "steam"))]
    pub fn new(app_id: u32) -> Arc<Self> {
        Arc::new(Self {
            status: Mutex::new(SteamStatus {
                is_connected: false,
                user_name: "Standalone Git Edition".to_string(),
                steam_id: "0".to_string(),
                app_id,
            }),
        })
    }

    pub fn set_wallpaper_presence(&self, _title: &str) {
        #[cfg(feature = "steam")]
        if let Some(ref client) = self.client {
            let status_msg = format!("Wallpaper: {}", _title);
            client.friends().set_rich_presence("status", Some(&status_msg));
        }
    }

    pub fn get_status(&self) -> SteamStatus {
        self.status.lock().unwrap().clone()
    }
}
