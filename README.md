# AetherPaper — Git Edition

A high-performance, modern animated wallpaper engine for Linux (KDE Plasma 6 on Wayland, universal Wayland compositors, and X11) featuring a clean monochromatic interface, adaptive video compression, and optional Steamworks integration.

---

## Features

### Performance & Video Compression
- **Quality Slider (Profiles 1 - 5)**:
  - **Level 1 (Max Performance)**: Aggressive compression downscaled to 720p at 24 FPS with CRF 28. Lowest possible GPU and battery draw.
  - **Level 2 (Economy)**: 1080p at 30 FPS with CRF 25. Smooth background playback for everyday efficiency.
  - **Level 3 (Balanced - Recommended)**: Crisp 1080p at 30/60 FPS with CRF 22. Optimal balance of visual fidelity and system overhead.
  - **Level 4 (High Fidelity)**: High bitrate 1440p/4K at 60 FPS with CRF 19. Maximum clarity for high-end displays.
  - **Level 5 (Raw / Source)**: Untouched source video preserving native frame rate, resolution, and bitrate.
- **Configurable Rendering FPS**:
  - Limit wallpaper playback to 15 FPS (Battery Saver), 24 FPS (Cinematic), 30 FPS (Smooth / Low GPU), 60 FPS (Fluid), or Native Source.
- **Hardware-Accelerated Decoding**:
  - Zero-copy hardware video decoding via VA-API and NVDEC.
- **Smart Game Detector**:
  - Automatically pauses animated wallpapers when fullscreen games or 3D applications become active, restoring normal playback on exit.

### Wallpaper Management & Persistent Storage
- **Predictable Local Storage**:
  - All imported wallpapers, optimized video streams, thumbnails, and library definitions are organized persistently in the application's `./wallpapers/` directory.
- **Inline Double-Click Renaming**:
  - Double-click any wallpaper title in your library to rename it directly in place.
- **Smooth QOL Installation Flow**:
  - Importing videos or subscribing to workshop items displays an immediate blurred preview with a thin white glowing scan line, transitioning smoothly to full clarity upon completion.
- **Reliable Thumbnail Previews**:
  - Base64 data streaming ensures instant, reliable thumbnail rendering across WebKitGTK without protocol or cross-origin restrictions.

### Multi-Backend Architecture
- **KDE Plasma 6 Wayland Native Engine**:
  - Evaluates wallpaper containments via Plasma D-Bus API, delivering zero-overhead hardware-accelerated playback with full desktop icon, widget, and right-click menu interaction.
- **Universal MPV Engine**:
  - Standalone hardware-accelerated background engine for Hyprland, Sway, GNOME, and X11 environments.

### Steam & Community Workshop
- Native Steamworks SDK client integration (`steamworks-rs`).
- Steam Rich Presence displays your active wallpaper on your Steam profile.
- Steam Workshop UGC support for community wallpapers.
- One-click desktop shortcut installer (`shortcuts.vdf`) generating custom capsules, hero artwork, and logos.
- System tray daemon keeping wallpapers active while allowing Steam games to run without interference.

---

## Quick Start

### Prerequisites
- Linux with KDE Plasma 6 (Wayland) or any compositor with `mpv`
- `ffmpeg` and `ffprobe` for video processing and frame extraction
- Node.js & `pnpm`
- Rust toolchain (`cargo`)

### Launching the Application
All lifecycle management scripts are located in `./setup/`:

```bash
# Launch optimized release binary:
./setup/launch.sh

# Or launch in Live Development mode with Hot Module Replacement:
./setup/launch.sh --dev
```

### Building Distribution Packages
```bash
# Build native binary into dist/:
./setup/build_image.sh

# Build container image:
./setup/build_image.sh --docker
```

### Adding to Steam
```bash
./setup/install-steam-shortcut.sh
```
Restart Steam to see AetherPaper in your Steam Library.

---

## Project Structure

```
AnimatedWallpaper/
├── src-tauri/                 # Rust backend and system integrations
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── steam_appid.txt        # Development Steam App ID (480)
│   └── src/
│       ├── main.rs            # Application entrypoint & IPC commands
│       ├── steam/             # Steamworks SDK, Workshop & shortcuts
│       ├── wallpaper/         # KDE Plasma 6 D-Bus & MPV engines
│       ├── monitors/          # Multi-screen detection
│       ├── performance/       # Fullscreen game detector
│       └── tray/              # System tray daemon
├── src/                       # Frontend application (Clean Grey theme)
│   ├── main.js                # Core UI logic, state, and IPC dispatch
│   ├── icons.js               # Featherweight custom SVG icon suite
│   └── styles/
│       └── theme.css          # Monochromatic theme & animations
├── setup/                     # Lifecycle scripts (launch, build, shortcut)
├── wallpapers/                # Persistent application wallpaper storage
│   └── thumbnails/            # Generated thumbnail cache
├── steam_assets/              # Official Steam Library artwork assets
└── assets/sample_wallpapers/  # Bundled sample looping videos
```

---

## Performance Tips

1. **On Battery / Laptops**: Set the Quality Slider to **Level 1** or **Level 2** and cap the Target Rendering FPS to **24 FPS** or **30 FPS**.
2. **Library Compression**: Click **"Compress Entire Library"** in the Settings tab to re-encode all wallpapers to the selected quality profile, drastically lowering storage consumption and decoding overhead.
3. **Gaming**: Keep **"Pause animated wallpaper when fullscreen games/apps are active"** checked in Settings for maximum frame rates in games.

---

## License
MIT
