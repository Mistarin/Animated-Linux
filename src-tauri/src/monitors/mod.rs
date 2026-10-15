use crate::wallpaper::backend::ScreenInfo;
use crate::new_host_command;

pub struct MonitorDetector;

impl MonitorDetector {
    pub fn detect_monitors() -> Vec<ScreenInfo> {
        let mut screens = Vec::new();

        // 1. Try querying Plasma Shell desktops
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
