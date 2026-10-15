use super::backend::{PlaybackSettings, WallpaperBackend, WallpaperItem};
use std::process::Child;
use crate::new_host_command;
use std::sync::Mutex;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

const MPV_SOCKET: &str = "/tmp/aetherpaper-mpv.sock";

pub struct MpvBackend {
    process: Mutex<Option<Child>>,
}

impl MpvBackend {
    pub fn new() -> Self {
        Self {
            process: Mutex::new(None),
        }
    }

    pub fn find_mpvpaper_bin() -> Option<PathBuf> {
        // 1. Check project bin/mpvpaper
        if let Ok(cwd) = std::env::current_dir() {
            let p = cwd.join("bin").join("mpvpaper");
            if p.exists() {
                return Some(p);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let p = parent.join("bin").join("mpvpaper");
                if p.exists() {
                    return Some(p);
                }
                if let Some(grandparent) = parent.parent() {
                    let p = grandparent.join("bin").join("mpvpaper");
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
        // 2. Check ~/.local/bin/mpvpaper
        if let Some(home) = dirs::home_dir() {
            let p = home.join(".local").join("bin").join("mpvpaper");
            if p.exists() {
                return Some(p);
            }
        }
        // 3. Check system PATH
        if let Ok(output) = new_host_command("which").arg("mpvpaper").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
        None
    }

    fn send_ipc_command(command: &str) -> Result<(), String> {
        let mut stream = UnixStream::connect(MPV_SOCKET)
            .map_err(|e| format!("Could not connect to mpv socket: {}", e))?;
        stream.write_all(format!("{}\n", command).as_bytes())
            .map_err(|e| format!("Failed to write to mpv socket: {}", e))?;
        Ok(())
    }
}

impl WallpaperBackend for MpvBackend {
    fn name(&self) -> &'static str {
        "MPV Zero-Copy Engine"
    }

    fn is_available(&self) -> bool {
        Self::find_mpvpaper_bin().is_some()
            || new_host_command("which").arg("mpv").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn apply(&self, wallpaper: &WallpaperItem, screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let mut proc_lock = self.process.lock().unwrap();

        // Kill existing mpv/mpvpaper if running
        if let Some(mut child) = proc_lock.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        let _ = std::fs::remove_file(MPV_SOCKET);

        let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        let mpvpaper_bin = Self::find_mpvpaper_bin();

        let child = if is_wayland && mpvpaper_bin.is_some() {
            let bin = mpvpaper_bin.unwrap();
            let mut cmd = new_host_command(bin);

            // Layer: bottom places the wallpaper directly beneath application windows and above desktop background
            cmd.arg("-l").arg("bottom");

            // Automatic pause policies for Wayland compositors
            match settings.pause_policy {
                1 => { cmd.arg("-p").arg("-a").arg("FULL"); }
                2 | 3 => { cmd.arg("-p").arg("-a").arg("MAX"); }
                _ => {}
            }

            // Build space-separated MPV options for mpvpaper -o
            let mut mpv_opts = Vec::new();
            if settings.hardware_accel {
                mpv_opts.push("hwdec=auto-safe".to_string());
            } else {
                mpv_opts.push("hwdec=no".to_string());
            }

            mpv_opts.push("loop=inf".to_string());
            mpv_opts.push(format!("input-ipc-server={}", MPV_SOCKET));
            mpv_opts.push(format!("mute={}", if settings.is_muted { "yes" } else { "no" }));
            mpv_opts.push(format!("volume={}", settings.volume));
            mpv_opts.push(format!("speed={}", settings.speed));
            if settings.is_paused {
                mpv_opts.push("pause=yes".to_string());
            }

            if settings.target_fps > 0 {
                mpv_opts.push(format!("vf=fps={}", settings.target_fps));
                mpv_opts.push(format!("display-fps-override={}", settings.target_fps));
            }

            // Scaling and render fidelity
            match settings.quality_level {
                1 => {
                    mpv_opts.push("scale=bilinear".to_string());
                    mpv_opts.push("dscale=bilinear".to_string());
                    mpv_opts.push("cscale=bilinear".to_string());
                    mpv_opts.push("vd-lavc-fast".to_string());
                    mpv_opts.push("vd-lavc-skiploopfilter=all".to_string());
                    mpv_opts.push("deband=no".to_string());
                    mpv_opts.push("dither=no".to_string());
                }
                2 => {
                    mpv_opts.push("scale=bilinear".to_string());
                    mpv_opts.push("vd-lavc-fast".to_string());
                    mpv_opts.push("deband=no".to_string());
                }
                3 => {
                    mpv_opts.push("scale=spline36".to_string());
                }
                4 => {
                    mpv_opts.push("scale=spline36".to_string());
                    mpv_opts.push("deband=yes".to_string());
                }
                5 => {
                    mpv_opts.push("scale=ewa_lanczossharp".to_string());
                    mpv_opts.push("deband=yes".to_string());
                }
                _ => {}
            }

            // Geometry and aspect ratio
            match settings.fill_mode {
                0 => { mpv_opts.push("keepaspect=no".to_string()); }
                1 => {
                    mpv_opts.push("keepaspect=yes".to_string());
                    mpv_opts.push("panscan=0.0".to_string());
                }
                2 => {
                    mpv_opts.push("keepaspect=yes".to_string());
                    mpv_opts.push("panscan=1.0".to_string());
                }
                3 => {
                    mpv_opts.push("keepaspect=yes".to_string());
                    mpv_opts.push("panscan=0.0".to_string());
                    mpv_opts.push("video-unscaled=yes".to_string());
                }
                _ => {}
            }

            mpv_opts.push(format!("video-pan-x={}", settings.pan_x));
            mpv_opts.push(format!("video-pan-y={}", settings.pan_y));

            cmd.arg("-o").arg(mpv_opts.join(" "));

            // Target output: '*' spans all connected Wayland outputs
            let target_output = if let Some(_s) = screen {
                "*".to_string()
            } else {
                "*".to_string()
            };
            cmd.arg(target_output);
            cmd.arg(&wallpaper.file_path);

            cmd.spawn().map_err(|e| format!("Failed to start mpvpaper: {}", e))?
        } else {
            // Standalone MPV fallback (X11 / embedded window)
            let mute_flag = if settings.is_muted { "yes" } else { "no" };
            let volume_str = settings.volume.to_string();
            let speed_str = settings.speed.to_string();

            let mut cmd = new_host_command("mpv");
            cmd.arg(&wallpaper.file_path)
                .arg("--loop=inf")
                .arg("--no-audio")
                .arg(format!("--mute={}", mute_flag))
                .arg(format!("--volume={}", volume_str))
                .arg(format!("--speed={}", speed_str))
                .arg(format!("--input-ipc-server={}", MPV_SOCKET))
                .arg("--wid=0");

            if settings.hardware_accel {
                cmd.arg("--hwdec=auto-safe");
            } else {
                cmd.arg("--hwdec=no");
            }

            if is_wayland {
                cmd.arg("--vo=dmabuf-wayland,gpu-next,gpu");
            } else {
                cmd.arg("--vo=gpu-next,gpu");
            }

            if settings.target_fps > 0 {
                cmd.arg(format!("--vf=fps={}", settings.target_fps));
                cmd.arg(format!("--display-fps-override={}", settings.target_fps));
            }

            match settings.quality_level {
                1 => {
                    cmd.arg("--scale=bilinear")
                        .arg("--dscale=bilinear")
                        .arg("--cscale=bilinear")
                        .arg("--vd-lavc-fast")
                        .arg("--vd-lavc-skiploopfilter=all")
                        .arg("--deband=no")
                        .arg("--dither=no");
                }
                2 => {
                    cmd.arg("--scale=bilinear")
                        .arg("--vd-lavc-fast")
                        .arg("--deband=no");
                }
                3 => {
                    cmd.arg("--scale=spline36");
                }
                4 => {
                    cmd.arg("--scale=spline36")
                        .arg("--deband=yes");
                }
                5 => {
                    cmd.arg("--scale=ewa_lanczossharp")
                        .arg("--deband=yes");
                }
                _ => {}
            }

            match settings.fill_mode {
                0 => { cmd.arg("--keepaspect=no"); }
                1 => { cmd.arg("--keepaspect=yes").arg("--panscan=0.0"); }
                2 => { cmd.arg("--keepaspect=yes").arg("--panscan=1.0"); }
                3 => { cmd.arg("--keepaspect=yes").arg("--panscan=0.0").arg("--video-unscaled=yes"); }
                _ => {}
            }
            cmd.arg(format!("--video-pan-x={}", settings.pan_x));
            cmd.arg(format!("--video-pan-y={}", settings.pan_y));

            cmd.spawn().map_err(|e| format!("Failed to start mpv: {}", e))?
        };

        *proc_lock = Some(child);
        Ok(())
    }

    fn pause(&self, _screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let _ = Self::send_ipc_command(r#"{"command": ["set_property", "pause", true]}"#);
        if settings.inactive_black_screen {
            let _ = Self::send_ipc_command(r#"{"command": ["set_property", "vid", "no"]}"#);
        }
        Ok(())
    }

    fn resume(&self, _screen: Option<i32>, settings: &PlaybackSettings, _wallpaper: Option<&WallpaperItem>) -> Result<(), String> {
        if settings.inactive_black_screen {
            let _ = Self::send_ipc_command(r#"{"command": ["set_property", "vid", "auto"]}"#);
        }
        if settings.restart_on_resume {
            let _ = Self::send_ipc_command(r#"{"command": ["seek", 0, "absolute"]}"#);
        }
        Self::send_ipc_command(r#"{"command": ["set_property", "pause", false]}"#)
    }

    fn set_volume(&self, volume: u32, muted: bool) -> Result<(), String> {
        let mute_bool = if muted { "true" } else { "false" };
        let _ = Self::send_ipc_command(&format!(r#"{{"command": ["set_property", "mute", {}]}}"#, mute_bool));
        Self::send_ipc_command(&format!(r#"{{"command": ["set_property", "volume", {}]}}"#, volume))
    }

    fn set_speed(&self, speed: f32) -> Result<(), String> {
        Self::send_ipc_command(&format!(r#"{{"command": ["set_property", "speed", {}]}}"#, speed))
    }

    fn set_geometry(&self, fill_mode: u32, pan_x: f32, pan_y: f32) -> Result<(), String> {
        let (keepaspect, panscan) = match fill_mode {
            0 => ("false", "0.0"),
            1 => ("true", "0.0"),
            2 => ("true", "1.0"),
            _ => ("true", "0.0"),
        };
        let _ = Self::send_ipc_command(&format!(r#"{{"command": ["set_property", "keepaspect", {}]}}"#, keepaspect));
        let _ = Self::send_ipc_command(&format!(r#"{{"command": ["set_property", "panscan", {}]}}"#, panscan));
        let _ = Self::send_ipc_command(&format!(r#"{{"command": ["set_property", "video-pan-x", {}]}}"#, pan_x));
        let _ = Self::send_ipc_command(&format!(r#"{{"command": ["set_property", "video-pan-y", {}]}}"#, pan_y));
        Ok(())
    }

    fn stop(&self, _screen: Option<i32>, _settings: &PlaybackSettings) -> Result<(), String> {
        let mut proc_lock = self.process.lock().unwrap();
        if let Some(mut child) = proc_lock.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(MPV_SOCKET);
        Ok(())
    }
}
