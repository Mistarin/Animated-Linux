use std::process::Command;

pub struct GameDetector;

impl GameDetector {
    /// Checks if a fullscreen window is currently active on KDE Plasma / KWin
    pub fn is_fullscreen_app_active() -> bool {
        let output_res = Command::new("qdbus")
            .arg("org.kde.plasmashell")
            .arg("/PlasmaShell")
            .arg("org.kde.PlasmaShell.evaluateScript")
            .arg("windowModel && windowModel.maximizedExists ? 'true' : 'false'")
            .output();

        if let Ok(output) = output_res {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return stdout.trim() == "true";
            }
        }
        false
    }
}
