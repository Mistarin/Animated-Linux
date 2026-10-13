use super::backend::{PlaybackSettings, WallpaperBackend, WallpaperItem};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::io::Write;
use std::os::unix::net::UnixStream;

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
        "MPV Hardware Accelerated Engine"
    }

    fn is_available(&self) -> bool {
        Command::new("which").arg("mpv").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn apply(&self, wallpaper: &WallpaperItem, _screen: Option<i32>, settings: &PlaybackSettings) -> Result<(), String> {
        let mut proc_lock = self.process.lock().unwrap();

        // Kill existing mpv if running
        if let Some(mut child) = proc_lock.take() {
            let _ = child.kill();
        }

        let _ = std::fs::remove_file(MPV_SOCKET);

        let mute_flag = if settings.is_muted { "yes" } else { "no" };
        let volume_str = settings.volume.to_string();
        let speed_str = settings.speed.to_string();

        let mut cmd = Command::new("mpv");
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

        // FPS limiting to drastically reduce CPU/GPU draw cycles
        if settings.target_fps > 0 {
            cmd.arg(format!("--vf=fps={}", settings.target_fps));
            cmd.arg(format!("--override-display-fps={}", settings.target_fps));
        }

        // Performance scaling & quality profiles
        match settings.quality_level {
            1 => {
                // Max Performance: lowest GPU load, fast bilinear scaling, skip loop filter
                cmd.arg("--scale=bilinear")
                    .arg("--dscale=bilinear")
                    .arg("--cscale=bilinear")
                    .arg("--vd-lavc-fast")
                    .arg("--vd-lavc-skiploopfilter=all")
                    .arg("--deband=no")
                    .arg("--dither=no");
            }
            2 => {
                // Economy: fast bilinear scaling, lavc fast
                cmd.arg("--scale=bilinear")
                    .arg("--vd-lavc-fast")
                    .arg("--deband=no");
            }
            3 => {
                // Balanced: smooth spline36, normal filtering
                cmd.arg("--scale=spline36");
            }
            4 => {
                // High Fidelity: high quality spline36 with debanding
                cmd.arg("--scale=spline36")
                    .arg("--deband=yes");
            }
            5 => {
                // Raw / Max Quality: ewa_lanczossharp with debanding
                cmd.arg("--scale=ewa_lanczossharp")
                    .arg("--deband=yes");
            }
            _ => {}
        }

        let child = cmd.spawn().map_err(|e| format!("Failed to start mpv: {}", e))?;

        *proc_lock = Some(child);
        Ok(())
    }

    fn pause(&self, _screen: Option<i32>) -> Result<(), String> {
        Self::send_ipc_command(r#"{"command": ["set_property", "pause", true]}"#)
    }

    fn resume(&self, _screen: Option<i32>) -> Result<(), String> {
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

    fn stop(&self, _screen: Option<i32>) -> Result<(), String> {
        let mut proc_lock = self.process.lock().unwrap();
        if let Some(mut child) = proc_lock.take() {
            let _ = child.kill();
        }
        let _ = std::fs::remove_file(MPV_SOCKET);
        Ok(())
    }
}
