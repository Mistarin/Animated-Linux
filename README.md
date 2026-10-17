# Animated Linux

<p align="center">
  <a href="https://github.com/Mistarin/Animated-Linux">
    <img src="https://img.shields.io/badge/Platform-Linux-FCC624?style=flat-square&logo=linux&logoColor=black" alt="Platform Linux" />
  </a>
  <a href="https://github.com/Mistarin/Animated-Linux">
    <img src="https://img.shields.io/badge/Desktop-KDE%20Plasma%206-1D99F3?style=flat-square&logo=kde&logoColor=white" alt="KDE Plasma 6" />
  </a>
  <a href="https://github.com/Mistarin/Animated-Linux">
    <img src="https://img.shields.io/badge/Display-Wayland%20%2F%20X11-2b2b36?style=flat-square" alt="Display Server" />
  </a>
  <a href="https://github.com/Mistarin/Animated-Linux/stargazers">
    <img src="https://img.shields.io/github/stars/Mistarin/Animated-Linux?style=flat-square&logo=github&label=Stars&color=10b981" alt="GitHub Stars" />
  </a>
  <a href="https://github.com/Mistarin/Animated-Linux/watchers">
    <img src="https://img.shields.io/github/watchers/Mistarin/Animated-Linux?style=flat-square&logo=github&label=Favorites&color=6366f1" alt="GitHub Favorites" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-GPL%203.0-blue.svg?style=flat-square" alt="License GPL 3.0" />
  </a>
  <a href="https://tauri.app">
    <img src="https://img.shields.io/badge/Built%20with-Rust%20%26%20Tauri-DEA584?style=flat-square&logo=rust" alt="Rust & Tauri" />
  </a>
</p>

Animated wallpaper manager for Linux with native KDE Plasma 6 D-Bus containment integration and a universal hardware-accelerated MPV backend. Features per-wallpaper resolution variant caching (Q1–Q5), runtime video pan and sizing controls, and zero idle overhead.

---

## Architecture

- **KDE Plasma 6 Wayland Engine**: Evaluates wallpaper containments via Plasma D-Bus API (`org.kde.PlasmaShell.evaluateScript`). Runs inside the native shell renderer with full desktop icon, widget, and context menu support.
- **Universal MPV Engine**: Hardware-accelerated background layer runner with IPC controls for Wayland compositors (Hyprland, Sway, GNOME) and X11.
- **Tauri 2 Core**: Memory footprint under 30MB of RAM compared to Electron-based engines.

---

## Features

### Quality Scale & Variant Caching

Wallpapers automatically cache generated variants in `./wallpapers/` (`{id}_q1.mp4` through `{id}_q5.mp4`). Dragging the Quality Slider switches instantly between cached variants with zero transcoding delay. If an active quality level is not yet cached, the UI highlights an action to generate it on demand.

| Profile | Target Resolution | Frame Rate | Codec Preset | Use Case |
| :--- | :--- | :--- | :--- | :--- |
| **Q5 (Native)** | Source Resolution | Native | Pass-through | Untouched source video file |
| **Q4 (4K)** | 3840x2160 | 60 FPS | libx264 (CRF 18) | High-resolution UHD displays |
| **Q3 (2K)** | 2560x1440 | 60 FPS | libx264 (CRF 21) | 1440p monitors |
| **Q2 (FULLHD)** | 1920x1080 | 30 FPS | libx264 (CRF 24) | Standard 1080p desktop |
| **Q1 (HD)** | 1280x720 or lower | 24 FPS | libx264 (CRF 28) | Battery saver & low-spec GPUs |

### Video Sizing & Pan Alignment
- **Sizing Modes**: `Fill / Zoom` (expand to fill), `Fit Aspect` (letterboxed), `Stretch`, and `Center` (1:1 unscaled).
- **Pan Offsets**: Real-time horizontal (`X: -50% to +50%`) and vertical (`Y: -50% to +50%`) adjustments with a quick reset button.
- Updates are applied live via IPC to active backends without restarting playback.

### Configurable Pausing Policies
- **Fullscreen Game / App**: Automatically detects running 3D games and fullscreen apps via window state monitoring and pauses playback.
- **Maximized Window Active**: Pauses when any window is maximized.
- **Any Window Active / Covered**: Pauses when the desktop layer is obscured.
- **Never Pause**: Continuous playback.
- **Battery Power**: Pauses when running on battery.

### Storage & Interface
- **Self-Contained Storage**: All imported videos, converted variants, thumbnails, and `library.json` remain inside the `./wallpapers/` application folder.
- **Base64 Previews**: Thumbnails streamed as Base64 data URLs to prevent WebKitGTK asset protocol restrictions.
- **Inline Renaming**: Double-click any wallpaper title to edit in place.
- **Clean Dark Theme**: High-contrast zinc palette with Inter typography and custom SVG icons.

---

## Dependencies

- **Linux Kernel** & **systemd**
- **KDE Plasma 6** (for native Plasma backend) or **mpv** (for universal Wayland/X11 backend)
- **ffmpeg** & **ffprobe** (for frame extraction and video scaling)
- **Node.js** >= 18 & **pnpm**
- **Rust** >= 1.75

---

## Building & Installation

### 1. Clone Repository
```bash
git clone https://github.com/Mistarin/Animated-Linux.git
cd Animated-Linux
```

### 2. Install Dependencies
```bash
pnpm install
```

### 3. Run Directly From Source
```bash
./launch.sh --dev
```

### 4. Build Standalone Executable (Optional)
```bash
./launch.sh --build
```
Compiles a single native executable at `dist/animated-linux-x86_64` without packaging.

### 5. Launch Executable
```bash
./launch.sh
```

### 6. Install Desktop Menu Entry
```bash
./launch.sh --install-desktop
```

---

## Project Structure

```
Animated-Linux/
├── src-tauri/                 # Rust core & backend bridges
│   ├── Cargo.toml             # Rust package configuration
│   ├── tauri.conf.json        # Tauri window & bundle settings
│   └── src/
│       ├── main.rs            # IPC command dispatch & detector loop
│       ├── wallpaper/         # Plasma 6 D-Bus & MPV engines
│       ├── monitors/          # Display detection
│       ├── performance/       # Fullscreen game detector
│       └── tray/              # System tray daemon
├── src/                       # Frontend application
│   ├── main.js                # UI state & IPC handlers
│   ├── icons.js               # SVG icon set
│   └── styles/
│       └── theme.css          # Dark zinc stylesheet
├── setup/                     # Build & launch scripts
│   ├── launch.sh              # Production / dev launcher
│   └── build_image.sh         # Binary & container builder
├── wallpapers/                # Local persistent wallpaper storage
│   └── thumbnails/            # Thumbnail cache
└── assets/sample_wallpapers/  # Bundled sample loops
```

---

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
