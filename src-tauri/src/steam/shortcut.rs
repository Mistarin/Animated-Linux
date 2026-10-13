use std::path::{Path, PathBuf};
use std::fs;

pub struct SteamShortcutInstaller;

impl SteamShortcutInstaller {
    /// Discovers all Steam user directories
    pub fn find_steam_user_dirs() -> Vec<PathBuf> {
        let base_steam = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Steam/userdata");

        let mut user_dirs = Vec::new();
        if let Ok(entries) = fs::read_dir(&base_steam) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if name_str != "anonymous" && name_str.chars().all(|c| c.is_ascii_digit()) {
                        user_dirs.push(path);
                    }
                }
            }
        }
        user_dirs
    }

    /// Computes Steam shortcut ID from exe and appname (Valve standard CRC algorithm)
    pub fn compute_shortcut_id(exe: &str, app_name: &str) -> u32 {
        let key = format!("{}{}", exe, app_name);
        let mut crc: u32 = 0xFFFFFFFF;
        for byte in key.bytes() {
            crc ^= byte as u32;
            for _ in 0..8 {
                let mask = if (crc & 1) != 0 { 0xEDB88320 } else { 0 };
                crc = (crc >> 1) ^ mask;
            }
        }
        (crc ^ 0xFFFFFFFF) | 0x80000000
    }

    /// Automatically installs the app shortcut into Steam library
    pub fn install_to_steam(app_exe: &Path, app_name: &str, app_dir: &Path) -> Result<usize, String> {
        let user_dirs = Self::find_steam_user_dirs();
        if user_dirs.is_empty() {
            return Err("No active Steam user accounts detected in ~/.local/share/Steam/userdata/".to_string());
        }

        let exe_str = format!("\"{}\"", app_exe.to_string_lossy());
        let dir_str = format!("\"{}\"", app_dir.to_string_lossy());
        let icon_path = app_dir.join("src-tauri/icons/icon.png");
        let shortcut_id = Self::compute_shortcut_id(&exe_str, app_name);

        let mut count = 0;
        for user_dir in &user_dirs {
            let config_dir = user_dir.join("config");
            let grid_dir = config_dir.join("grid");
            let _ = fs::create_dir_all(&grid_dir);

            // Copy Steam artwork assets
            let src_grid = app_dir.join("steam_assets/grid.png");
            let src_hero = app_dir.join("steam_assets/hero.png");

            if src_grid.exists() {
                let _ = fs::copy(&src_grid, grid_dir.join(format!("{}p.png", shortcut_id)));
                let _ = fs::copy(&src_grid, grid_dir.join(format!("{}.png", shortcut_id)));
            }
            if src_hero.exists() {
                let _ = fs::copy(&src_hero, grid_dir.join(format!("{}_hero.png", shortcut_id)));
            }
            if icon_path.exists() {
                let _ = fs::copy(&icon_path, grid_dir.join(format!("{}_logo.png", shortcut_id)));
            }

            let shortcuts_file = config_dir.join("shortcuts.vdf");
            let mut shortcuts_bytes = if shortcuts_file.exists() {
                fs::read(&shortcuts_file).unwrap_or_else(|_| Self::empty_vdf())
            } else {
                Self::empty_vdf()
            };

            // Check if already registered
            let content_str = String::from_utf8_lossy(&shortcuts_bytes);
            if !content_str.contains(app_name) {
                // Append new entry to binary VDF
                shortcuts_bytes = Self::append_shortcut_vdf(
                    &shortcuts_bytes,
                    shortcut_id,
                    app_name,
                    &exe_str,
                    &dir_str,
                    &icon_path.to_string_lossy(),
                );
                let _ = fs::write(&shortcuts_file, &shortcuts_bytes);
            }
            count += 1;
        }

        Ok(count)
    }

    fn empty_vdf() -> Vec<u8> {
        let mut v = Vec::new();
        v.push(0x00);
        v.extend_from_slice(b"shortcuts\x00");
        v.push(0x08);
        v.push(0x08);
        v
    }

    fn append_shortcut_vdf(
        existing: &[u8],
        appid: u32,
        name: &str,
        exe: &str,
        start_dir: &str,
        icon: &str,
    ) -> Vec<u8> {
        let mut out = existing.to_vec();
        // Remove trailing \x08\x08 if present
        while out.ends_with(&[0x08]) {
            out.pop();
        }

        // Count existing entries
        let mut index = 0;
        let s = String::from_utf8_lossy(&out);
        while s.contains(&format!("\x00{}\x00", index)) {
            index += 1;
        }

        out.push(0x00);
        out.extend_from_slice(format!("{}\x00", index).as_bytes());

        // appid
        out.push(0x02);
        out.extend_from_slice(b"appid\x00");
        out.extend_from_slice(&appid.to_le_bytes());

        // AppName
        out.push(0x01);
        out.extend_from_slice(b"AppName\x00");
        out.extend_from_slice(name.as_bytes());
        out.push(0x00);

        // Exe
        out.push(0x01);
        out.extend_from_slice(b"Exe\x00");
        out.extend_from_slice(exe.as_bytes());
        out.push(0x00);

        // StartDir
        out.push(0x01);
        out.extend_from_slice(b"StartDir\x00");
        out.extend_from_slice(start_dir.as_bytes());
        out.push(0x00);

        // icon
        out.push(0x01);
        out.extend_from_slice(b"icon\x00");
        out.extend_from_slice(icon.as_bytes());
        out.push(0x00);

        // AllowOverlay = 1
        out.push(0x02);
        out.extend_from_slice(b"AllowOverlay\x00");
        out.extend_from_slice(&1u32.to_le_bytes());

        // tags
        out.push(0x00);
        out.extend_from_slice(b"tags\x00");
        out.push(0x01);
        out.extend_from_slice(b"0\x00Utilities\x00");
        out.push(0x01);
        out.extend_from_slice(b"1\x00Wallpapers\x00");
        out.push(0x08);

        // End of entry and end of shortcuts list
        out.push(0x08);
        out.push(0x08);
        out.push(0x08);

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_shortcut_id() {
        let id = SteamShortcutInstaller::compute_shortcut_id("/usr/bin/aetherpaper", "AetherPaper");
        assert_ne!(id, 0);
        assert!(id >= 0x80000000);
    }

    #[test]
    fn test_find_steam_users() {
        let dirs = SteamShortcutInstaller::find_steam_user_dirs();
        assert!(!dirs.is_empty(), "Should detect local Steam user directories");
    }

    #[test]
    fn test_vdf_generation() {
        let empty = SteamShortcutInstaller::empty_vdf();
        assert!(empty.len() > 10);
        let populated = SteamShortcutInstaller::append_shortcut_vdf(
            &empty,
            12345678,
            "TestApp",
            "/bin/test",
            "/bin",
            "/icon.png"
        );
        let s = String::from_utf8_lossy(&populated);
        assert!(s.contains("TestApp"));
        assert!(s.contains("Utilities"));
    }

    #[test]
    fn test_install_to_steam() {
        let exe = Path::new("/home/martin/Main/Programming/AnimatedWallpaper/src-tauri/target/debug/aetherpaper");
        let app_dir = Path::new("/home/martin/Main/Programming/AnimatedWallpaper");
        let res = SteamShortcutInstaller::install_to_steam(exe, "AetherPaper", app_dir);
        assert!(res.is_ok(), "Steam shortcut installation should succeed: {:?}", res);
    }
}


