use crate::wallpaper::backend::ScreenInfo;
use crate::new_host_command;

pub struct MonitorDetector;

impl MonitorDetector {
    pub fn detect_monitors() -> Vec<ScreenInfo> {
        let mut screens = Vec::new();

        // 1. Try querying kscreen-doctor JSON for precise connector names and resolutions
        if let Ok(output) = new_host_command("kscreen-doctor").arg("-j").output() {
            if output.status.success() {
                if let Ok(json_str) = String::from_utf8(output.stdout) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if let Some(outputs) = val.get("outputs").and_then(|o| o.as_array()) {
                            let mut idx = 0;
                            for out in outputs {
                                let connected = out.get("connected").and_then(|c| c.as_bool()).unwrap_or(false);
                                let enabled = out.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                                if connected && enabled {
                                    let connector = out.get("name").and_then(|n| n.as_str()).unwrap_or("Display").to_string();
                                    let width = out.get("size").and_then(|s| s.get("width")).and_then(|w| w.as_i64()).unwrap_or(1920);
                                    let height = out.get("size").and_then(|s| s.get("height")).and_then(|h| h.as_i64()).unwrap_or(1080);
                                    screens.push(ScreenInfo {
                                        id: idx,
                                        name: connector,
                                        resolution: format!("{}x{}", width, height),
                                        current_wallpaper: None,
                                    });
                                    idx += 1;
                                }
                            }
                            if !screens.is_empty() {
                                return screens;
                            }
                        }
                    }
                }
            }
        }

        // 2. Try querying Plasma Shell desktops
        if let Ok(output) = new_host_command("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .arg("org.kde.PlasmaShell.evaluateScript")
            .arg("desktops().map(d => d.screen).join(',')")
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let indices: Vec<&str> = stdout.trim().split(',').filter(|s| !s.is_empty()).collect();
                if !indices.is_empty() {
                    for (i, &idx_str) in indices.iter().enumerate() {
                        let screen_id = idx_str.parse::<i32>().unwrap_or(i as i32);
                        screens.push(ScreenInfo {
                            id: screen_id,
                            name: format!("Display {}", screen_id + 1),
                            resolution: "Primary Display".to_string(),
                            current_wallpaper: None,
                        });
                    }
                    return screens;
                }
            }
        }

        // Fallback default screen
        screens.push(ScreenInfo {
            id: 0,
            name: "Primary Monitor".to_string(),
            resolution: "1920x1080".to_string(),
            current_wallpaper: None,
        });

        screens
    }
}
