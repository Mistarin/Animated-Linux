# Animated Linux System Architecture & KDE Plasma Integration

## 1. Document Purpose and Scope

This document defines the architectural foundations, design decisions, and strict invariants for Animated Linux (`AnimatedWallpaper`). It serves as the authoritative technical reference to ensure that performance, stability, and desktop integration remain robust across releases and to prevent regression loops.

---

## 2. System Overview and Technology Stack

Animated Linux is a high-performance live wallpaper and video backdrop manager engineered specifically for Linux desktop environments, with primary focus on KDE Plasma 6 running on Wayland and X11.

### 2.1 Core Components

```
+-----------------------------------------------------------------------+
|                         Frontend Layer (Web)                          |
|  Vanilla JavaScript + Vite + HTML5 / CSS3                             |
|  - UI State Management, Displays, Library, Settings                   |
|  - Zero Heavy Framework Overhead                                      |
+-----------------------------------------------------------------------+
                                   |
                         Tauri 2 IPC (invoke)
                                   v
+-----------------------------------------------------------------------+
|                          Host Core (Rust)                             |
|  src-tauri/src/main.rs                                                |
|  - AppState: Settings persistence, Window Management, Native IPC      |
|                                                                       |
|  src-tauri/src/wallpaper/mod.rs: WallpaperManager                     |
|  - Library management, resolution profiles (Q1-Q5), video metadata    |
|                                                                       |
|  src-tauri/src/wallpaper/request_manager.rs: RequestManager           |
|  - Per-screen serial execution queues (MPSC channels)                 |
|  - Atomic state transitions, debounce, and D-Bus concurrency guards   |
+-----------------------------------------------------------------------+
                                   |
                       WallpaperBackend Trait
                                   v
+-----------------------------------------------------------------------+
|                      KDE Plasma 6 Backend                             |
|  src-tauri/src/wallpaper/kde_plasma.rs                                |
|  - D-Bus PlasmaShell Script Evaluator                                 |
|  - User-space Plugin Mirror & In-Plugin Patch Engine                  |
|  - Native QML FastBlur & Hardware Overlay Controller                  |
+-----------------------------------------------------------------------+
                                   |
                      org.kde.plasmashell D-Bus
                                   v
+-----------------------------------------------------------------------+
|                  KDE Plasma Shell Desktop Containment                 |
|  luisbocanegra.smart.video.wallpaper.reborn                           |
|  QtQuick / QtMultimedia / Hardware Acceleration Pipeline             |
+-----------------------------------------------------------------------+
```

### 2.2 Key File Locations

- Backend Core: [src-tauri/src/wallpaper/mod.rs](file:///home/martin/Main/Programming/AnimatedWallpaper/src-tauri/src/wallpaper/mod.rs)
- KDE Plasma Implementation: [src-tauri/src/wallpaper/kde_plasma.rs](file:///home/martin/Main/Programming/AnimatedWallpaper/src-tauri/src/wallpaper/kde_plasma.rs)
- Concurrency Serialization: [src-tauri/src/wallpaper/request_manager.rs](file:///home/martin/Main/Programming/AnimatedWallpaper/src-tauri/src/wallpaper/request_manager.rs)
- Frontend Logic: [src/main.js](file:///home/martin/Main/Programming/AnimatedWallpaper/src/main.js)
- UI Definition: [index.html](file:///home/martin/Main/Programming/AnimatedWallpaper/index.html)

---

## 3. The RequestManager Concurrency Model

### 3.1 The Problem
In KDE Plasma 6 Wayland, desktop containments process wallpaper configuration updates asynchronously over D-Bus via `org.kde.PlasmaShell.evaluateScript`. If multiple updates are issued concurrently (for example, rapid window focus shifts, mouse movements, or settings adjustments), script evaluations race inside plasmashell. This causes configuration corruption, dropped commands, and race conditions where the video player pipeline gets wedged between conflicting states.

### 3.2 The Solution
All wallpaper state transitions are dispatched through `RequestManager`:
1. Every display output is assigned a dedicated worker thread with an unbuffered or shallow `tokio::sync::mpsc` channel.
2. Requests for a given screen are strictly serialized.
3. Rapid succession commands are debounced so only the latest valid request executes.
4. Each D-Bus script evaluation completes synchronously before the next request for that display is dispatched.

---

## 4. KDE Plasma 6 Desktop Model and Scripting

### 4.1 Containment and Desktops
KDE Plasma manages desktop workspaces as containments. In Plasma scripting:
- `desktops()` returns an array of desktop containment objects.
- Each containment object `d` has a `.screen` property indicating the display ID.
- Target `-1` indicates all screens; non-negative numbers target a specific monitor.
- Configuration is accessed through `d.currentConfigGroup = Array("Wallpaper", pluginId, "General")`.
- Changes must be persisted via `d.writeConfig(key, value)` followed by `d.reloadConfig()`.

### 4.2 Script Execution Protocol
Scripts are executed over D-Bus using:
```bash
qdbus org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript "<script-contents>"
```

---

## 5. The "Plugin-Swap Trap" (Core Anti-Pattern & Root Cause Analysis)

### 5.1 Historical Failure Mode
In earlier revisions, the backend attempted to implement "fade to black", "pause", and "stop preview" by dynamically changing `d.wallpaperPlugin` between `luisbocanegra.smart.video.wallpaper.reborn` and static color/image plugins (`org.kde.color` or `org.kde.image`).

This resulted in a chronic failure loop:
1. Swapping `wallpaperPlugin` forces plasmashell to tear down the entire QML scene graph and destroy the underlying QtMultimedia / GStreamer / VAAPI hardware decoding pipeline.
2. Tearing down the player while video surfaces are rendering causes Wayland buffer release races and GPU driver synchronization stalls.
3. When the user subsequently resumed playback or switched wallpapers, plasmashell re-instantiated the QML scene graph. However, because QML item creation, video demuxing, and hardware decoder negotiation are asynchronous, the video player frequently initialized into an invalid or frozen state.
4. The wallpaper appeared stuck on a black screen, failed to render frames, or broke window occlusion detection.
5. In attempts to fix the black screen, additional kickstarts or plugin swaps were introduced, worsening the synchronization race.

### 5.2 Architectural Resolution
**Under no circumstances should `d.wallpaperPlugin` be dynamically swapped to another plugin during pause, idle, or fade states.**
Once an animated wallpaper is active, `d.wallpaperPlugin` remains permanently locked to `luisbocanegra.smart.video.wallpaper.reborn`. All visual transitions (black fading, blurring, frame freezing) must occur **inside** the plugin scene graph without destroying the media player tree.

---

## 6. In-Plugin Hardware Overlay and FastBlur Architecture

To satisfy visual requirements without tearing down the media player, Animated Linux uses in-plugin GPU primitives.

### 6.1 Inactive Fade-to-Black (`blackOverlay`)
Rather than switching to a black desktop plugin, a hardware `Rectangle` is injected directly into `main.qml` of the Reborn plugin:

```qml
Rectangle {
    id: blackOverlay
    anchors.fill: parent
    color: "#000000"
    opacity: (main.inactiveBlackScreen && !main.playing && main.videosConfig.length > 0) ? 1.0 : 0.0
    visible: opacity > 0.001 || (main.inactiveBlackScreen && !main.playing && main.videosConfig.length > 0)
    z: 99
    Behavior on opacity {
        NumberAnimation {
            duration: 350
            easing.type: Easing.InOutQuad
        }
    }
}
```

- When the video is playing, `opacity` is `0.0`.
- When the video pauses due to window occlusion, inactivity, or manual pause, `opacity` smoothly transitions to `1.0` over 350ms using a hardware-accelerated `InOutQuad` easing curve.
- The underlying QtMultimedia player stays loaded, alive, and ready to render the first frame immediately upon resumption.

### 6.2 Inactive Fade-to-Blur (`FastBlur`)
Smart Video Wallpaper Reborn natively contains a `FastBlur` element (`mainBlur`) layered over the video background:

- The blur activation is governed by configuration key `BlurMode`.
- In Reborn's QML enum, `BlurMode = 3` corresponds to `Enum.BlurMode.VideoPaused`, triggering whenever `!main.playing` evaluates to true.
- Configuration key `BlurRadius` controls the blur strength (1 to 64 px).
- Reborn provides a native 300ms transition animation on `mainBlur.radius`.
- When "Fade to blur" is enabled:
  - `BlurMode` is set to `3`.
  - `BlurRadius` is set to `settings.blur_strength` (default: 32).
- When "Fade to blur" is disabled:
  - `BlurMode` is set to `5` (`Enum.BlurMode.Never`).
  - `BlurRadius` is set to `0`.

### 6.3 Combined Operation
Fade-to-Black and Fade-to-Blur can be used independently or combined:
- **Blur only**: The video smoothly softens into a frosted glass background when covered or paused.
- **Black only**: The video smoothly darkens to pure black when covered or paused.
- **Both**: The video blurs while simultaneously darkening to black for a cinematic vignette effect.
- **Neither**: The video crisply pauses and holds the current frame.

---

## 7. The 2-Stage Playback Kickstart

### 7.1 The QtQuick Signal Dropping Problem
When writing configuration updates to `luisbocanegra.smart.video.wallpaper.reborn`, changing `PauseMode` directly from one active mode to another does not always trigger a property mutation notification in the QML engine if the numerical condition inside Reborn does not generate a discrete edge transition. As a result, the video player would remain paused even when an active wallpaper was applied or resumed.

### 7.2 The 2-Stage Transition Protocol
To guarantee that `onPlayingChanged` fires deterministically, `apply()` and `resume()` execute a two-stage sequence:

1. **Stage 1 (State Reset)**: Write `PauseMode = 99` (an unused out-of-bounds mode) and trigger `d.reloadConfig()`.
2. **Yield**: Sleep for 30 milliseconds to allow QtQuick event loop to register the mode transition.
3. **Stage 2 (Target Configuration)**: Write the target `PauseMode` (0, 1, or 3) along with video URLs and effects, followed by `d.reloadConfig()`.

This forces an explicit edge trigger in QML, ensuring that `main.play()` is invoked reliably without UI hangs or dropped frames.

---

## 8. Accurate Reset-on-Resume (0:00 Restart)

### 8.1 Reborn Media Load Limitation
Smart Video Wallpaper Reborn provides configuration entries `ResumeLastVideo` and `LastVideoPosition`. However, Reborn only evaluates these parameters during the initial media loading phase. Once a video is loaded and paused mid-playback, writing `LastVideoPosition = 0` to configuration has zero effect on the active player instance.

### 8.2 The Injected Player Position Reset
To support the feature "Restart animation from beginning on resume", Animated Linux patches `pauseTimer` and `playTimer` in `main.qml`:

```qml
// In pauseTimer:
onTriggered: {
    player.pause();
    if (main.restartOnResume) {
        try {
            player.player.position = 0;
        } catch(e) {}
    }
}

// In playTimer:
onTriggered: {
    if (main.restartOnResume) {
        try {
            player.player.position = 0;
        } catch(e) {}
    }
    player.play();
}
```

This guarantees that whenever playback resumes after an occlusion pause, the video begins at timestamp 0:00 rather than continuing from the middle of an animation loop.

---

## 9. User-Space Plugin Mirroring (`ensure_local_patch`)

### 9.1 Background
The system plugin directory `/usr/share/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn` is owned by `root`. Modifying files in `/usr/share` requires elevated permissions and is overwritten whenever system packages update.

### 9.2 The Automatic Mirror Mechanism
The backend function `KdePlasmaBackend::ensure_local_patch()`:
1. Inspects `~/.local/share/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn`.
2. If absent, recursively copies the system plugin from `/usr/share/plasma/wallpapers/` into user space.
3. Verifies that `contents/config/main.xml` includes `InactiveBlackScreen` and `RestartOnResume`. If missing, injects them.
4. Verifies that `contents/ui/main.qml` contains the `blackOverlay` Rectangle and position reset logic. If missing, injects them.
5. If modifications were written, executes `kbuildsycoca6 --noincremental` once to rebuild the Plasma system configuration cache.

This ensures all required capabilities are present without requiring sudo privileges or risk of package manager conflicts.

---

## 10. Strict Invariants for Future Maintainers

Any future modifications to Animated Linux must adhere to the following non-negotiable rules:

1. **NO DYNAMIC PLUGIN SWAPPING**: Do not switch `d.wallpaperPlugin` to `org.kde.color`, `org.kde.image`, or any other plugin during pause, resume, or inactivity. `d.wallpaperPlugin` is only switched to `org.kde.image` when the user explicitly clicks "Reset Desktop to Default".
2. **SERIALIZE VIA REQUESTMANAGER**: All commands that interact with D-Bus, plasmashell, or wallpaper backend state must pass through `RequestManager`. Never invoke direct D-Bus commands from asynchronous tasks without queue serialization.
3. **PRESERVE THE 2-STAGE KICKSTART**: Do not eliminate the `PauseMode: 99` transition in `apply()` and `resume()`. It is required to prevent QtQuick signal dropping in Plasma 6.
4. **NO EMOJIS OR DINGBATS**: Strictly zero emojis, symbols, or pictograms in source code, comments, UI text, logs, commit messages, or documentation. Clean, professional typography only.
5. **PRESERVE GPL-3.0 LICENSING**: Animated Linux and its components are licensed under the GNU General Public License v3.0. Preserve license headers on all distributed files.
6. **USE file:// LINKS IN MARKDOWN**: Internal file references in project markdown documentation must strictly use the standard `file://` URI scheme.
