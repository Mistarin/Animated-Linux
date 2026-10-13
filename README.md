# Animated Linux

A high-performance, modern animated wallpaper engine for Linux (KDE Plasma 6 on Wayland, universal Wayland compositors, and X11) featuring a clean monochromatic interface, per-wallpaper variant caching, video sizing and pan positioning controls, and granular pause policies.

---

## Features

### Performance & Per-Wallpaper Variant Caching
- **Quality Scale (Q1 - Q5)**:
  - **Q5 (Native)**: Untouched original source video preserving native resolution, frame rate, and original bitrate.
  - **Q4 (4K)**: Ultra high-definition 4K encoding (`3840x2160`) at 60 FPS (`CRF 18`).
  - **Q3 (2K)**: Quad HD 1440p encoding (`2560x1440`) at 60 FPS (`CRF 21`).
  - **Q2 (FULLHD)**: Full HD 1080p encoding (`1920x1080`) at 30 FPS (`CRF 24`).
  - **Q1 (HD or lower)**: Downscaled 720p encoding (`1280x720`) at 24 FPS (`CRF 28`). Minimal GPU load and maximum battery saving.
- **Variant Caching (1 - 5)**:
  - Each wallpaper automatically caches its rendered variants (`{id}_q1.mp4` through `{id}_q5.mp4`) in `./wallpapers/`.
  - When switching the quality slider, if the active wallpaper already has that variant cached, it switches instantly with zero transcode delay.
  - If the variant is not yet cached, the **"Optimize & Cache"** button turns vibrant green, indicating that clicking it will generate and cache the variant.
- **Configurable Rendering FPS**:
  - Limit wallpaper playback to 15 FPS (Battery Saver), 24 FPS (Cinematic), 30 FPS (Smooth / Low GPU), 60 FPS (Fluid), or Native Source.
- **Granular Pausing Policies**:
  - Fullscreen Game / App (Performance default)
  - Maximized Window Active
  - Any Window Active / Covered
  - Never Pause (Always Play)
  - Battery Power (Laptops / Handhelds)
- **Hardware-Accelerated Decoding**:
  - Hardware video decoding via VA-API and NVDEC.

### Video Sizing & Pan Position Controls
- **Sizing Modes**:
  - Fill / Zoom (Expand to fill screen while preserving aspect ratio)
  - Fit Aspect (Fit entire video inside screen boundaries)
  - Stretch (Stretch video to completely fill screen)
  - Center (1:1 unscaled centered video)
- **Pan Offsets (X & Y)**:
  - Sliders allowing horizontal and vertical repositioning of the video layer with one-click zero reset.

### Wallpaper Management & Persistent Storage
- **Predictable Local Storage**:
  - All wallpapers, cached quality variants, thumbnails, and library definitions are organized persistently in `./wallpapers/`.
- **Inline Double-Click Renaming**:
  - Double-click any wallpaper title in your library to rename it in place.
- **Smooth QOL Installation Flow**:
  - Importing videos displays an immediate blurred preview with a thin white glowing scan line, transitioning smoothly to full clarity upon completion.
- **Reliable Base64 Previews**:
  - Base64 data streaming ensures instant, reliable thumbnail rendering across WebKitGTK without protocol or cross-origin restrictions.

### Multi-Backend Architecture
- **KDE Plasma 6 Wayland Native Engine**:
  - Evaluates wallpaper containments via Plasma D-Bus API, delivering zero-overhead hardware-accelerated playback with full desktop icon, widget, and right-click menu interaction.
- **Universal MPV Engine**:
  - Standalone hardware-accelerated background engine for Hyprland, Sway, GNOME, and X11 environments.

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
# Launch release binary:
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

---

## Project Structure

```
AnimatedWallpaper/
├── src-tauri/                 # Rust backend and system integrations
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs            # Application entrypoint & IPC commands
│       ├── wallpaper/         # KDE Plasma 6 D-Bus & MPV engines
│       ├── monitors/          # Multi-screen detection
│       ├── performance/       # Fullscreen game detector
│       └── tray/              # System tray daemon
├── src/                       # Frontend application (Clean Grey theme)
│   ├── main.js                # Core UI logic, state, and IPC dispatch
│   ├── icons.js               # Featherweight custom SVG icon suite
│   └── styles/
│       └── theme.css          # Monochromatic theme & animations
├── setup/                     # Lifecycle scripts (launch, build)
├── wallpapers/                # Persistent application wallpaper storage
│   └── thumbnails/            # Generated thumbnail cache
└── assets/sample_wallpapers/  # Bundled sample looping videos
```

---

## License
MIT
