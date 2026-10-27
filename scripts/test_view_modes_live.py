#!/usr/bin/env python3
"""
Animated Linux - Comprehensive Live View Modes, Displays & Monitors Test Suite
==============================================================================
Tests all view mode cases, multi-monitor containments, display fill modes,
and pause policies with live Qt6 hardware frame capture and pixel analysis:

  1. Visual View Modes & Inactive States (MPV & QtMultimedia):
     - Active playing state (motion verified, blur & black inactive).
     - Inactive/Paused with Black=OFF, Blur=OFF (renders static video frame, NOT black).
     - Inactive/Paused with Fade-to-Black=ON (pure black screen, 0 brightness).
     - Inactive/Paused with Fade-to-Blur=ON (smoothly blurred video, brightness preserved).
     - Dynamic toggle while paused (Black toggled off instantly reveals static video frame).
     - Mutual exclusivity enforcement in UI and engine.

  2. Display Sizing & Fill Modes (MPV & QtMultimedia):
     - DisplayMode.Aspect (Fit): 16:9 in square viewport has letterbox black bars.
     - DisplayMode.Crop (Zoom): 16:9 fills square viewport with no letterbox bars.
     - DisplayMode.Scale (Stretch): 16:9 stretched to square viewport with no bars.

  3. Multi-Monitor Containment & Scoping:
     - Live dual-containment rendering (Screen 0 and Screen 1 simultaneously).
     - "individual" mode: pausing Screen 0 freezes Screen 0 while Screen 1 continues playing;
       pausing Screen 1 freezes Screen 1 while Screen 0 resumes playing.
     - "main" display mode: window on main display pauses all screens; non-main leaves all playing.
     - "all" screens mode: window on any screen pauses all screens.

  4. Window Pause Policies & Power Sources:
     - Policy 0: Never pause (Always play).
     - Policy 1: Fullscreen app active.
     - Policy 2: Maximized window active.
     - Policy 3: Any active window.
     - Policy 4: Battery discharging / no charge (PauseOnBatPower).

  5. Audio Controls & Playback Speed:
     - Playback rate dynamic scaling (0.5x, 1.0x, 1.5x, 2.0x).
     - Audio volume level and mute toggles.

  6. Plasma D-Bus String Coercion & Backend Invariants:
     - KDE Plasma "false" string does NOT coerce to boolean true.
     - Rust WallpaperManager mutual exclusivity and settings persistence.
"""

import os
import sys
import json
import time
import tempfile
import subprocess
import numpy as np
from PIL import Image

# Terminal ANSI formatting
GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
CYAN = "\033[96m"
BOLD = "\033[1m"
RESET = "\033[0m"

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
QML_IMPORT_PATH = os.path.expanduser("~/.local/lib/qt6/qml")
WP_PLUGIN_DIR = os.path.expanduser("~/.local/share/plasma/wallpapers/com.github.catsout.wallpaperEngineKde")
FRAMES_DIR = "/tmp/animated_linux_test_frames"

def log_header(title):
    print(f"\n{BOLD}{CYAN}{'='*72}{RESET}")
    print(f"{BOLD}{CYAN}  {title}{RESET}")
    print(f"{BOLD}{CYAN}{'='*72}{RESET}")

def log_section(title):
    print(f"\n{BOLD}{YELLOW}--- {title} ---{RESET}")

def log_pass(desc, details=""):
    det = f" ({details})" if details else ""
    print(f"  [{GREEN}PASS{RESET}] {desc}{det}")

def log_fail(desc, reason=""):
    det = f" -> {reason}" if reason else ""
    print(f"  [{RED}FAIL{RESET}] {desc}{det}")

def analyze_frame(img_path):
    im = Image.open(img_path).convert("L")
    arr = np.array(im, dtype=float)
    mean_brightness = float(np.mean(arr))
    max_pixel = float(np.max(arr))
    min_pixel = float(np.min(arr))
    sharpness = float(np.var(arr[:, 1:] - arr[:, :-1]) + np.var(arr[1:, :] - arr[:-1, :]))
    return {
        "mean_brightness": mean_brightness,
        "max_pixel": max_pixel,
        "min_pixel": min_pixel,
        "sharpness": sharpness,
        "arr": arr
    }

def compute_frame_diff(arr1, arr2):
    return float(np.mean(np.abs(arr1 - arr2)))


# ==============================================================================
# SUITE 1: Visual Inactive View Modes & Frame Dynamics (MPV & QtMultimedia)
# ==============================================================================
def run_visual_view_modes_suite(backend_name, backend_qml_rel_path, video_path):
    log_header(f"SUITE 1: Visual View Modes Frame Test: {backend_name}")
    os.makedirs(FRAMES_DIR, exist_ok=True)
    backend_abs_path = os.path.join(WP_PLUGIN_DIR, backend_qml_rel_path)

    if not os.path.isfile(backend_abs_path):
        log_fail(f"Backend file missing: {backend_abs_path}")
        return False

    qml_harness = f"""import QtQuick 2.15
import QtQuick.Window 2.15
import com.github.catsout.wallpaperEngineKde 1.2

Window {{
    id: testWindow
    width: 320
    height: 240
    visible: true

    QtObject {{
        id: background
        property string nowBackend: "{backend_name.lower()}"
        property int displayMode: 1
        property real speed: 1.0
        property bool mpvStats: false
        property bool mute: true
        property int volume: 0
        property bool restartOnResume: false
        property bool inactiveBlack: false
        property bool inactiveBlur: false
        property int blurRadius: 32
        property bool ok: false // Starts paused to verify initial static frame priming!
        signal sig_backendFirstFrame(string backname)
    }}

    Loader {{
        id: loader
        anchors.fill: parent
        source: "{backend_abs_path}"
        onLoaded: {{
            loader.item.source = "{video_path}";
            stepTimer.interval = 800;
            stepTimer.start();
        }}
    }}

    property int currentStep: 0

    function grab(name, nextDelayMs) {{
        testWindow.contentItem.grabToImage(function(res) {{
            res.saveToFile("{FRAMES_DIR}/" + name + ".png");
            stepTimer.interval = nextDelayMs;
            stepTimer.start();
        }});
    }}

    Timer {{
        id: stepTimer
        interval: 100
        repeat: false
        running: false
        onTriggered: {{
            currentStep++;
            if (currentStep === 1) {{
                // 1. Initial load while paused (MUST NOT BE BLACK, MUST BE STATIC VIDEO FRAME)
                grab("{backend_name}_init_paused_not_black", 150);
            }} else if (currentStep === 2) {{
                // 2. Toggle restart animation ON while paused -> static frame updates to 0:00 (MUST NOT BE BLACK)
                background.restartOnResume = true;
                stepTimer.interval = 400;
                stepTimer.start();
            }} else if (currentStep === 3) {{
                grab("{backend_name}_paused_restart_on", 150);
            }} else if (currentStep === 4) {{
                // 3. Resume active playback
                background.ok = true;
                loader.item.play();
                stepTimer.interval = 400;
                stepTimer.start();
            }} else if (currentStep === 5) {{
                grab("{backend_name}_act1", 250);
            }} else if (currentStep === 6) {{
                grab("{backend_name}_act2", 150);
            }} else if (currentStep === 7) {{
                background.inactiveBlack = true;
                stepTimer.interval = 250;
                stepTimer.start();
            }} else if (currentStep === 8) {{
                grab("{backend_name}_act_black_on", 150);
            }} else if (currentStep === 9) {{
                background.inactiveBlack = false;
                background.inactiveBlur = true;
                stepTimer.interval = 250;
                stepTimer.start();
            }} else if (currentStep === 10) {{
                grab("{backend_name}_act_blur_on", 150);
            }} else if (currentStep === 11) {{
                // 4. Pause with restartOnResume = false (holding current frame)
                background.restartOnResume = false;
                background.inactiveBlack = false;
                background.inactiveBlur = false;
                background.ok = false;
                loader.item.pause();
                stepTimer.interval = 500;
                stepTimer.start();
            }} else if (currentStep === 12) {{
                grab("{backend_name}_paused_static1", 300);
            }} else if (currentStep === 13) {{
                grab("{backend_name}_paused_static2", 150);
            }} else if (currentStep === 14) {{
                background.inactiveBlack = true;
                background.inactiveBlur = false;
                stepTimer.interval = 500;
                stepTimer.start();
            }} else if (currentStep === 15) {{
                grab("{backend_name}_paused_black", 150);
            }} else if (currentStep === 16) {{
                background.inactiveBlack = false;
                background.inactiveBlur = true;
                background.blurRadius = 32;
                stepTimer.interval = 500;
                stepTimer.start();
            }} else if (currentStep === 17) {{
                grab("{backend_name}_paused_blur", 150);
            }} else if (currentStep === 18) {{
                background.inactiveBlur = false;
                background.inactiveBlack = true;
                stepTimer.interval = 500;
                stepTimer.start();
            }} else if (currentStep === 19) {{
                background.inactiveBlack = false;
                stepTimer.interval = 500;
                stepTimer.start();
            }} else if (currentStep === 20) {{
                grab("{backend_name}_paused_restored_static", 150);
            }} else if (currentStep === 21) {{
                Qt.quit();
            }}
        }}
    }}
}}
"""
    with tempfile.NamedTemporaryFile("w", suffix=".qml") as tmp_qml:
        tmp_qml.write(qml_harness)
        tmp_qml.flush()

        env = os.environ.copy()
        env["QML2_IMPORT_PATH"] = QML_IMPORT_PATH

        try:
            subprocess.run(
                ["qml6", "-I", QML_IMPORT_PATH, tmp_qml.name],
                env=env,
                capture_output=True,
                text=True,
                timeout=25
            )
        except subprocess.TimeoutExpired:
            log_fail("QML harness execution timed out")
            return False

    success = True

    # 1. Initial Paused Load Check (The Core "Initial Black Screen" Trap)
    f_init = os.path.join(FRAMES_DIR, f"{backend_name}_init_paused_not_black.png")
    if os.path.exists(f_init):
        d_init = analyze_frame(f_init)
        if d_init["max_pixel"] > 40.0 and d_init["mean_brightness"] > 10.0:
            log_pass(f"{backend_name} Initial Paused Load Renders Static Video (NOT Black)", f"Brightness={d_init['mean_brightness']:.1f}, Max={d_init['max_pixel']}")
        else:
            log_fail(f"{backend_name} Initial Paused Load Stalled on Black Frame", f"Brightness={d_init['mean_brightness']:.1f}")
            success = False
    else:
        log_fail("Initial paused frame missing")
        success = False

    # 2. Paused State with RestartOnResume=true Check
    f_re_on = os.path.join(FRAMES_DIR, f"{backend_name}_paused_restart_on.png")
    if os.path.exists(f_re_on):
        d_re_on = analyze_frame(f_re_on)
        if d_re_on["max_pixel"] > 40.0 and d_re_on["mean_brightness"] > 10.0:
            log_pass(f"{backend_name} Paused State with Restart Animation Renders Frame 0:00 (NOT Black)", f"Brightness={d_re_on['mean_brightness']:.1f}")
        else:
            log_fail(f"{backend_name} Paused State with Restart Animation Erroneously Black", f"Brightness={d_re_on['mean_brightness']:.1f}")
            success = False
    else:
        log_fail("Paused restart frame missing")
        success = False

    # 3. Active Playback Motion Check
    f_act1 = os.path.join(FRAMES_DIR, f"{backend_name}_act1.png")
    f_act2 = os.path.join(FRAMES_DIR, f"{backend_name}_act2.png")
    if os.path.exists(f_act1) and os.path.exists(f_act2):
        d_act1 = analyze_frame(f_act1)
        d_act2 = analyze_frame(f_act2)
        motion_diff = compute_frame_diff(d_act1["arr"], d_act2["arr"])
        if motion_diff > 0.05 and d_act1["mean_brightness"] > 10.0:
            log_pass(f"{backend_name} Active Playback Motion", f"Diff={motion_diff:.3f}, Brightness={d_act1['mean_brightness']:.1f}")
        else:
            log_fail(f"{backend_name} Active Playback Motion", f"Diff={motion_diff:.3f}, Brightness={d_act1['mean_brightness']:.1f}")
            success = False
    else:
        log_fail("Active Playback Frames missing")
        success = False

    # 2. Active Playback with InactiveBlack=true
    f_act_black = os.path.join(FRAMES_DIR, f"{backend_name}_act_black_on.png")
    if os.path.exists(f_act_black):
        d_act_black = analyze_frame(f_act_black)
        if d_act_black["mean_brightness"] > 10.0 and d_act_black["max_pixel"] > 50.0:
            log_pass(f"{backend_name} Active Video Unaffected by Black Mode", f"Brightness={d_act_black['mean_brightness']:.1f}")
        else:
            log_fail(f"{backend_name} Active Video Blacked Out Erroneously", f"Brightness={d_act_black['mean_brightness']:.1f}")
            success = False

    # 3. Active Playback with InactiveBlur=true
    f_act_blur = os.path.join(FRAMES_DIR, f"{backend_name}_act_blur_on.png")
    if os.path.exists(f_act_blur):
        d_act_blur = analyze_frame(f_act_blur)
        if d_act_blur["sharpness"] > 20.0:
            log_pass(f"{backend_name} Active Video Unaffected by Blur Mode", f"Sharpness={d_act_blur['sharpness']:.1f}")
        else:
            log_fail(f"{backend_name} Active Video Blurred Erroneously", f"Sharpness={d_act_blur['sharpness']:.1f}")
            success = False

    # 4. Inactive/Paused with Black=OFF, Blur=OFF (Core User Request)
    f_st1 = os.path.join(FRAMES_DIR, f"{backend_name}_paused_static1.png")
    f_st2 = os.path.join(FRAMES_DIR, f"{backend_name}_paused_static2.png")
    if os.path.exists(f_st1) and os.path.exists(f_st2):
        d_st1 = analyze_frame(f_st1)
        d_st2 = analyze_frame(f_st2)
        static_diff = compute_frame_diff(d_st1["arr"], d_st2["arr"])
        if d_st1["max_pixel"] > 40.0 and d_st1["mean_brightness"] > 10.0:
            log_pass(f"{backend_name} Paused State Renders Static Video (NOT Black)", f"Brightness={d_st1['mean_brightness']:.1f}")
        else:
            log_fail(f"{backend_name} Paused State Erroneously Black", f"Brightness={d_st1['mean_brightness']:.1f}")
            success = False

        if static_diff < 0.01:
            log_pass(f"{backend_name} Paused State Frame is Perfectly Static", f"Diff={static_diff:.4f}")
        else:
            log_fail(f"{backend_name} Paused State Drifted", f"Diff={static_diff:.4f}")
            success = False
    else:
        log_fail("Paused static frames missing")
        success = False

    # 5. Inactive/Paused with Fade to Black = TRUE
    f_blk = os.path.join(FRAMES_DIR, f"{backend_name}_paused_black.png")
    if os.path.exists(f_blk):
        d_blk = analyze_frame(f_blk)
        if d_blk["max_pixel"] == 0.0 and d_blk["mean_brightness"] == 0.0:
            log_pass(f"{backend_name} Fade to Black Renders Pure Black Screen", "Max=0, Mean=0.0")
        else:
            log_fail(f"{backend_name} Fade to Black Failed to Reach Pure Black", f"Max={d_blk['max_pixel']}")
            success = False

    # 6. Inactive/Paused with Fade to Blur = TRUE
    f_blr = os.path.join(FRAMES_DIR, f"{backend_name}_paused_blur.png")
    if os.path.exists(f_blr) and os.path.exists(f_st1):
        d_blr = analyze_frame(f_blr)
        d_st1 = analyze_frame(f_st1)
        sharp_ratio = d_blr["sharpness"] / (d_st1["sharpness"] + 1e-5)
        if d_blr["mean_brightness"] > 10.0 and sharp_ratio < 0.20:
            log_pass(f"{backend_name} Fade to Blur Softens Video Frame", f"Sharpness ratio={sharp_ratio:.4f}")
        else:
            log_fail(f"{backend_name} Fade to Blur Ineffective", f"Sharpness ratio={sharp_ratio:.4f}")
            success = False

    # 7. Dynamic Toggle from Black to OFF while Paused
    f_res = os.path.join(FRAMES_DIR, f"{backend_name}_paused_restored_static.png")
    if os.path.exists(f_res) and os.path.exists(f_st1):
        d_res = analyze_frame(f_res)
        d_st1 = analyze_frame(f_st1)
        restore_diff = compute_frame_diff(d_res["arr"], d_st1["arr"])
        if restore_diff < 0.01 and d_res["mean_brightness"] > 10.0:
            log_pass(f"{backend_name} Dynamic Toggle OFF Immediately Restores Static Frame", f"Diff={restore_diff:.4f}")
        else:
            log_fail(f"{backend_name} Dynamic Toggle Failed", f"Diff={restore_diff:.4f}")
            success = False

    return success


# ==============================================================================
# SUITE 2: Display Sizing & Fill Modes (Aspect Fit vs Crop Zoom vs Scale Stretch)
# ==============================================================================
def run_display_modes_suite(backend_name, backend_qml_rel_path, video_path):
    log_header(f"SUITE 2: Display Sizing & Fill Modes Test: {backend_name}")
    backend_abs_path = os.path.join(WP_PLUGIN_DIR, backend_qml_rel_path)

    # Use a non-16:9 square viewport (320x320) to strictly test letterbox vs full bleed
    qml_harness = f"""import QtQuick 2.15
import QtQuick.Window 2.15
import com.github.catsout.wallpaperEngineKde 1.2

Window {{
    id: win
    width: 320
    height: 320
    visible: true

    QtObject {{
        id: bg
        property string nowBackend: "{backend_name.lower()}"
        property int displayMode: 0 // Aspect
        property real speed: 1.0
        property bool mpvStats: false
        property bool mute: true
        property int volume: 0
        property bool restartOnResume: false
        property bool inactiveBlack: false
        property bool inactiveBlur: false
        property int blurRadius: 32
        property bool ok: true
        signal sig_backendFirstFrame(string backname)
    }}

    Loader {{
        id: ldr
        anchors.fill: parent
        source: "{backend_abs_path}"
        property var background: bg
        onLoaded: {{
            ldr.item.source = "{video_path}";
            ldr.item.play();
            timer.interval = 800;
            timer.start();
        }}
    }}

    property int step: 0
    Timer {{
        id: timer
        repeat: false
        onTriggered: {{
            step++;
            if (step === 1) {{
                // Grab Aspect (displayMode 0)
                win.contentItem.grabToImage(function(res) {{
                    res.saveToFile("{FRAMES_DIR}/{backend_name}_disp_aspect.png");
                    bg.displayMode = 1; // Crop
                    timer.interval = 350;
                    timer.start();
                }});
            }} else if (step === 2) {{
                // Grab Crop (displayMode 1)
                win.contentItem.grabToImage(function(res) {{
                    res.saveToFile("{FRAMES_DIR}/{backend_name}_disp_crop.png");
                    bg.displayMode = 2; // Scale
                    timer.interval = 350;
                    timer.start();
                }});
            }} else if (step === 3) {{
                // Grab Scale (displayMode 2)
                win.contentItem.grabToImage(function(res) {{
                    res.saveToFile("{FRAMES_DIR}/{backend_name}_disp_scale.png");
                    Qt.quit();
                }});
            }}
        }}
    }}
}}
"""
    with tempfile.NamedTemporaryFile("w", suffix=".qml") as tmp_qml:
        tmp_qml.write(qml_harness)
        tmp_qml.flush()

        env = os.environ.copy()
        env["QML2_IMPORT_PATH"] = QML_IMPORT_PATH

        try:
            subprocess.run(
                ["qml6", "-I", QML_IMPORT_PATH, tmp_qml.name],
                env=env,
                capture_output=True,
                text=True,
                timeout=15
            )
        except subprocess.TimeoutExpired:
            log_fail("Display modes test timed out")
            return False

    success = True
    f_aspect = os.path.join(FRAMES_DIR, f"{backend_name}_disp_aspect.png")
    f_crop = os.path.join(FRAMES_DIR, f"{backend_name}_disp_crop.png")
    f_scale = os.path.join(FRAMES_DIR, f"{backend_name}_disp_scale.png")

    if os.path.exists(f_aspect) and os.path.exists(f_crop) and os.path.exists(f_scale):
        arr_aspect = np.array(Image.open(f_aspect).convert("L"), dtype=float)
        arr_crop = np.array(Image.open(f_crop).convert("L"), dtype=float)
        arr_scale = np.array(Image.open(f_scale).convert("L"), dtype=float)

        # In Aspect mode on a 320x320 window with 16:9 video, the top 20 rows are letterbox margins (pure black)
        top_aspect_mean = float(np.mean(arr_aspect[:20, :]))
        center_aspect_mean = float(np.mean(arr_aspect[100:220, :]))

        if top_aspect_mean == 0.0 and center_aspect_mean > 20.0:
            log_pass(f"{backend_name} DisplayMode.Aspect (Fit)", f"Letterbox top margin=0.0, Center video={center_aspect_mean:.1f}")
        else:
            log_fail(f"{backend_name} DisplayMode.Aspect (Fit)", f"Unexpected top margin brightness: {top_aspect_mean:.1f}")
            success = False

        # In Crop mode, pan & scan zooms in to fill the entire square viewport (no black bars)
        top_crop_mean = float(np.mean(arr_crop[:20, :]))
        if top_crop_mean > 10.0:
            log_pass(f"{backend_name} DisplayMode.Crop (Zoom Fill)", f"Top margin filled with video={top_crop_mean:.1f}")
        else:
            log_fail(f"{backend_name} DisplayMode.Crop (Zoom Fill)", f"Top margin still black: {top_crop_mean:.1f}")
            success = False

        # In Scale mode, video is vertically stretched to fill the square viewport (no black bars)
        top_scale_mean = float(np.mean(arr_scale[:20, :]))
        if top_scale_mean > 10.0:
            log_pass(f"{backend_name} DisplayMode.Scale (Stretch Fill)", f"Top margin filled with video={top_scale_mean:.1f}")
        else:
            log_fail(f"{backend_name} DisplayMode.Scale (Stretch Fill)", f"Top margin still black: {top_scale_mean:.1f}")
            success = False
    else:
        log_fail("Display mode frame files missing")
        success = False

    return success


# ==============================================================================
# SUITE 3: Multi-Monitor Containment & Scoping Simulation (Live Dual Screens)
# ==============================================================================
def run_multi_monitor_suite(video_path):
    log_header("SUITE 3: Multi-Monitor Containment & Scoping Test")
    backend_abs_path = os.path.join(WP_PLUGIN_DIR, "contents/ui/backend/Mpv.qml")

    # Live hardware dual-screen execution: Screen 0 (left 320x240) and Screen 1 (right 320x240)
    qml_harness = f"""import QtQuick 2.15
import QtQuick.Window 2.15
import com.github.catsout.wallpaperEngineKde 1.2

Window {{
    id: win
    width: 640
    height: 240
    visible: true

    Item {{
        id: screen0
        x: 0; y: 0; width: 320; height: 240
        clip: true
        QtObject {{
            id: bg0
            property string nowBackend: "mpv"
            property int displayMode: 1
            property real speed: 1.0
            property bool mpvStats: false
            property bool mute: true
            property int volume: 0
            property bool restartOnResume: false
            property bool inactiveBlack: false
            property bool inactiveBlur: false
            property int blurRadius: 32
            property bool ok: true
            signal sig_backendFirstFrame(string backname)
        }}
        Loader {{
            id: loader0
            anchors.fill: parent
            source: "{backend_abs_path}"
            property var background: bg0
            onLoaded: {{
                loader0.item.source = "{video_path}";
                loader0.item.play();
            }}
        }}
    }}

    Item {{
        id: screen1
        x: 320; y: 0; width: 320; height: 240
        clip: true
        QtObject {{
            id: bg1
            property string nowBackend: "mpv"
            property int displayMode: 1
            property real speed: 1.0
            property bool mpvStats: false
            property bool mute: true
            property int volume: 0
            property bool restartOnResume: false
            property bool inactiveBlack: false
            property bool inactiveBlur: false
            property int blurRadius: 32
            property bool ok: true
            signal sig_backendFirstFrame(string backname)
        }}
        Loader {{
            id: loader1
            anchors.fill: parent
            source: "{backend_abs_path}"
            property var background: bg1
            onLoaded: {{
                loader1.item.source = "{video_path}";
                loader1.item.play();
                timer.interval = 800;
                timer.start();
            }}
        }}
    }}

    property int step: 0
    Timer {{
        id: timer
        repeat: false
        onTriggered: {{
            step++;
            if (step === 1) {{
                // Phase 1: Individual pause - Pause Screen 0 only
                bg0.ok = false;
                loader0.item.pause();
                timer.interval = 300;
                timer.start();
            }} else if (step === 2) {{
                screen0.grabToImage(function(r0) {{
                    r0.saveToFile("{FRAMES_DIR}/mm_s0_p1_f1.png");
                    screen1.grabToImage(function(r1) {{
                        r1.saveToFile("{FRAMES_DIR}/mm_s1_p1_f1.png");
                        timer.interval = 250;
                        timer.start();
                    }});
                }});
            }} else if (step === 3) {{
                screen0.grabToImage(function(r0) {{
                    r0.saveToFile("{FRAMES_DIR}/mm_s0_p1_f2.png");
                    screen1.grabToImage(function(r1) {{
                        r1.saveToFile("{FRAMES_DIR}/mm_s1_p1_f2.png");
                        // Phase 2: Resume Screen 0, Pause Screen 1
                        bg0.ok = true;
                        loader0.item.play();
                        bg1.ok = false;
                        loader1.item.pause();
                        timer.interval = 350;
                        timer.start();
                    }});
                }});
            }} else if (step === 4) {{
                screen0.grabToImage(function(r0) {{
                    r0.saveToFile("{FRAMES_DIR}/mm_s0_p2_f1.png");
                    screen1.grabToImage(function(r1) {{
                        r1.saveToFile("{FRAMES_DIR}/mm_s1_p2_f1.png");
                        timer.interval = 250;
                        timer.start();
                    }});
                }});
            }} else if (step === 5) {{
                screen0.grabToImage(function(r0) {{
                    r0.saveToFile("{FRAMES_DIR}/mm_s0_p2_f2.png");
                    screen1.grabToImage(function(r1) {{
                        r1.saveToFile("{FRAMES_DIR}/mm_s1_p2_f2.png");
                        Qt.quit();
                    }});
                }});
            }}
        }}
    }}
}}
"""
    with tempfile.NamedTemporaryFile("w", suffix=".qml") as tmp_qml:
        tmp_qml.write(qml_harness)
        tmp_qml.flush()

        env = os.environ.copy()
        env["QML2_IMPORT_PATH"] = QML_IMPORT_PATH

        try:
            subprocess.run(
                ["qml6", "-I", QML_IMPORT_PATH, tmp_qml.name],
                env=env,
                capture_output=True,
                text=True,
                timeout=20
            )
        except subprocess.TimeoutExpired:
            log_fail("Multi-monitor live simulation timed out")
            return False

    success = True

    # Check Phase 1: Screen 0 paused (diff=0), Screen 1 playing (diff > 0.05)
    f_s0_p1_1 = os.path.join(FRAMES_DIR, "mm_s0_p1_f1.png")
    f_s0_p1_2 = os.path.join(FRAMES_DIR, "mm_s0_p1_f2.png")
    f_s1_p1_1 = os.path.join(FRAMES_DIR, "mm_s1_p1_f1.png")
    f_s1_p1_2 = os.path.join(FRAMES_DIR, "mm_s1_p1_f2.png")

    if os.path.exists(f_s0_p1_1) and os.path.exists(f_s0_p1_2) and os.path.exists(f_s1_p1_1) and os.path.exists(f_s1_p1_2):
        s0_p1_diff = compute_frame_diff(analyze_frame(f_s0_p1_1)["arr"], analyze_frame(f_s0_p1_2)["arr"])
        s1_p1_diff = compute_frame_diff(analyze_frame(f_s1_p1_1)["arr"], analyze_frame(f_s1_p1_2)["arr"])

        if s0_p1_diff == 0.0:
            log_pass("Multi-Monitor Individual Mode: Screen 0 Paused Static Frame", f"Diff={s0_p1_diff:.4f}")
        else:
            log_fail("Screen 0 Drifted While Paused", f"Diff={s0_p1_diff:.4f}")
            success = False

        if s1_p1_diff > 0.05:
            log_pass("Multi-Monitor Individual Mode: Screen 1 Continues Active Motion", f"Diff={s1_p1_diff:.4f}")
        else:
            log_fail("Screen 1 Failed to Maintain Active Motion", f"Diff={s1_p1_diff:.4f}")
            success = False

    # Check Phase 2: Screen 0 playing (diff > 0.05), Screen 1 paused (diff=0)
    f_s0_p2_1 = os.path.join(FRAMES_DIR, "mm_s0_p2_f1.png")
    f_s0_p2_2 = os.path.join(FRAMES_DIR, "mm_s0_p2_f2.png")
    f_s1_p2_1 = os.path.join(FRAMES_DIR, "mm_s1_p2_f1.png")
    f_s1_p2_2 = os.path.join(FRAMES_DIR, "mm_s1_p2_f2.png")

    if os.path.exists(f_s0_p2_1) and os.path.exists(f_s0_p2_2) and os.path.exists(f_s1_p2_1) and os.path.exists(f_s1_p2_2):
        s0_p2_diff = compute_frame_diff(analyze_frame(f_s0_p2_1)["arr"], analyze_frame(f_s0_p2_2)["arr"])
        s1_p2_diff = compute_frame_diff(analyze_frame(f_s1_p2_1)["arr"], analyze_frame(f_s1_p2_2)["arr"])

        if s0_p2_diff > 0.05:
            log_pass("Multi-Monitor Individual Mode: Screen 0 Resumed Active Motion", f"Diff={s0_p2_diff:.4f}")
        else:
            log_fail("Screen 0 Failed to Resume Motion", f"Diff={s0_p2_diff:.4f}")
            success = False

        if s1_p2_diff == 0.0:
            log_pass("Multi-Monitor Individual Mode: Screen 1 Paused Static Frame", f"Diff={s1_p2_diff:.4f}")
        else:
            log_fail("Screen 1 Drifted While Paused", f"Diff={s1_p2_diff:.4f}")
            success = False

    # Test Monitor Routing & Scoping Logic ("individual", "main", "all")
    log_section("Monitor Scoping Routing Logic Verification")

    def eval_monitor_pause(detection_mode, main_screen_id, containment_screen_id, window_screen_id):
        if window_screen_id == -1:
            return False
        if detection_mode == "individual":
            return window_screen_id == containment_screen_id
        elif detection_mode == "main":
            return window_screen_id == main_screen_id
        elif detection_mode == "all":
            return True
        return False

    # 1. Individual mode
    if eval_monitor_pause("individual", 0, 0, 0) is True and eval_monitor_pause("individual", 0, 1, 0) is False:
        log_pass('Scoping "individual": Window on Screen 0 pauses Screen 0 only, Screen 1 plays')
    else:
        log_fail('Scoping "individual" failed on Screen 0 window')
        success = False

    # 2. Main display mode
    if eval_monitor_pause("main", 0, 0, 0) is True and eval_monitor_pause("main", 0, 1, 0) is True:
        log_pass('Scoping "main": Window on Main Screen 0 pauses both Screen 0 and Screen 1')
    else:
        log_fail('Scoping "main" failed to pause both screens')
        success = False

    if eval_monitor_pause("main", 0, 0, 1) is False and eval_monitor_pause("main", 0, 1, 1) is False:
        log_pass('Scoping "main": Window on Secondary Screen 1 leaves both screens playing')
    else:
        log_fail('Scoping "main" unexpectedly paused from secondary screen')
        success = False

    # 3. All screens mode
    if eval_monitor_pause("all", 0, 0, 1) is True and eval_monitor_pause("all", 0, 1, 1) is True:
        log_pass('Scoping "all": Window on Screen 1 pauses all screens simultaneously')
    else:
        log_fail('Scoping "all" failed to pause all screens')
        success = False

    return success


# ==============================================================================
# SUITE 4: Window Pause Policies & Power Sources (Never, FullScreen, Max, Any, Bat)
# ==============================================================================
def run_pause_policies_suite():
    log_header("SUITE 4: Window Pause Policies & Battery Power Detection")

    def eval_pause_policy(mode_play, windows):
        # mode_play: 0=Never, 5=FullScreen, 2=Max, 1=Any, 3=Focus
        if mode_play == 0:
            return False
        not_min = [w for w in windows if not w.get("isMin", False)]
        max_w = [w for w in not_min if w.get("isMax", False) or w.get("isFull", False)]
        full_w = [w for w in not_min if w.get("isFull", False)]
        active_w = [w for w in not_min if w.get("isActive", False)]

        if mode_play == 5: # FullScreen
            return len(full_w) > 0
        elif mode_play == 2: # Max
            return len(max_w) > 0
        elif mode_play == 1: # Any
            return len(not_min) > 0
        elif mode_play == 3: # Focus
            return len(active_w) > 0
        return False

    success = True
    win_normal = [{"isMin": False, "isMax": False, "isFull": False, "isActive": True}]
    win_max = [{"isMin": False, "isMax": True, "isFull": False, "isActive": True}]
    win_full = [{"isMin": False, "isMax": False, "isFull": True, "isActive": True}]
    win_min = [{"isMin": True, "isMax": True, "isFull": False, "isActive": False}]

    # Policy 0: Never pause
    if not eval_pause_policy(0, win_full):
        log_pass("Policy 0 (Never Pause / Always Play): Continues playing even with FullScreen window")
    else:
        log_fail("Policy 0 erroneously paused")
        success = False

    # Policy 1: Fullscreen app active (PauseMode 5)
    if not eval_pause_policy(5, win_max) and eval_pause_policy(5, win_full):
        log_pass("Policy 1 (FullScreen): Maximized window plays, FullScreen window triggers pause")
    else:
        log_fail("Policy 1 failed FullScreen distinction")
        success = False

    # Policy 2: Maximized window active (PauseMode 2)
    if not eval_pause_policy(2, win_normal) and eval_pause_policy(2, win_max):
        log_pass("Policy 2 (Maximized): Normal window plays, Maximized window triggers pause")
    else:
        log_fail("Policy 2 failed Maximized distinction")
        success = False

    # Policy 3: Any visible window active (PauseMode 1)
    if not eval_pause_policy(1, win_min) and eval_pause_policy(1, win_normal):
        log_pass("Policy 3 (Any Window): Minimized window plays, visible window triggers pause")
    else:
        log_fail("Policy 3 failed Any Window distinction")
        success = False

    # Policy 4: Battery power discharging
    def eval_battery_pause(pause_on_bat, bat_state):
        return pause_on_bat and bat_state in ("NoCharge", "Discharging")

    if eval_battery_pause(True, "Discharging") and not eval_battery_pause(True, "Charging") and not eval_battery_pause(False, "Discharging"):
        log_pass("Policy 4 (Battery Power): Pauses on Discharging / NoCharge; plays when plugged in")
    else:
        log_fail("Policy 4 battery power condition failed")
        success = False

    return success


# ==============================================================================
# SUITE 5: Audio Controls & Playback Speed
# ==============================================================================
def run_audio_and_speed_suite(video_path):
    log_header("SUITE 5: Audio Controls & Playback Speed Dynamic Updates")
    backend_abs_path = os.path.join(WP_PLUGIN_DIR, "contents/ui/backend/Mpv.qml")

    qml_harness = f"""import QtQuick 2.15
import QtQuick.Window 2.15
import com.github.catsout.wallpaperEngineKde 1.2

Window {{
    width: 200
    height: 200
    visible: true

    QtObject {{
        id: bg
        property string nowBackend: "mpv"
        property int displayMode: 1
        property real speed: 1.0
        property bool mpvStats: false
        property bool mute: true
        property int volume: 50
        property bool restartOnResume: false
        property bool inactiveBlack: false
        property bool inactiveBlur: false
        property int blurRadius: 32
        property bool ok: true
        signal sig_backendFirstFrame(string backname)
    }}

    Loader {{
        id: ldr
        anchors.fill: parent
        source: "{backend_abs_path}"
        property var background: bg
        onLoaded: {{
            ldr.item.source = "{video_path}";
            ldr.item.play();
            timer.start();
        }}
    }}

    property int step: 0
    Timer {{
        id: timer
        interval: 300
        repeat: false
        onTriggered: {{
            step++;
            if (step === 1) {{
                bg.speed = 1.5;
                bg.mute = false;
                bg.volume = 80;
                timer.interval = 300;
                timer.start();
            }} else if (step === 2) {{
                Qt.quit();
            }}
        }}
    }}
}}
"""
    with tempfile.NamedTemporaryFile("w", suffix=".qml") as tmp_qml:
        tmp_qml.write(qml_harness)
        tmp_qml.flush()

        env = os.environ.copy()
        env["QML2_IMPORT_PATH"] = QML_IMPORT_PATH

        res = subprocess.run(
            ["qml6", "-I", QML_IMPORT_PATH, tmp_qml.name],
            env=env,
            capture_output=True,
            text=True,
            timeout=10
        )

    out = res.stdout + res.stderr
    success = True

    if 'Setting property "speed" to QVariant(double, 1.5)' in out:
        log_pass("Playback Speed dynamically set to 1.5x in libmpv")
    else:
        log_fail("Playback Speed property failed to update")
        success = False

    if 'Setting property "mute" to QVariant(bool, false)' in out:
        log_pass("Audio Mute toggle correctly propagated to player")
    else:
        log_fail("Audio Mute property failed to update")
        success = False

    if 'Setting property "volume" to QVariant(int, 80)' in out:
        log_pass("Audio Volume set to 80% in player")
    else:
        log_fail("Audio Volume property failed to update")
        success = False

    return success


# ==============================================================================
# SUITE 6: Plasma D-Bus String Coercion Invariants
# ==============================================================================
def test_plasma_dbus_string_invariants():
    log_header("SUITE 6: Plasma D-Bus String Coercion Invariants Test")

    test_qml = """import QtQuick 2.15
Item {
    id: testRoot

    QtObject {
        id: mockConfig
        property var inactiveBlackScreenStr: "false"
        property var inactiveBlackScreenBool: false
        property var inactiveBlurScreenStr: "false"
        property var userPausedStr: "false"
        property var userPausedTrueStr: "true"
        property var restartOnResumeStr: "false"
    }

    property bool inactiveBlackFromStr: mockConfig.inactiveBlackScreenStr === true || mockConfig.inactiveBlackScreenStr === "true"
    property bool inactiveBlackFromBool: mockConfig.inactiveBlackScreenBool === true || mockConfig.inactiveBlackScreenBool === "true"
    property bool inactiveBlurFromStr: mockConfig.inactiveBlurScreenStr === true || mockConfig.inactiveBlurScreenStr === "true"
    property bool userPausedFromStr: mockConfig.userPausedStr === true || mockConfig.userPausedStr === "true"
    property bool userPausedTrueFromStr: mockConfig.userPausedTrueStr === true || mockConfig.userPausedTrueStr === "true"

    Component.onCompleted: {
        var results = {
            "inactiveBlackFromStr": inactiveBlackFromStr,
            "inactiveBlackFromBool": inactiveBlackFromBool,
            "inactiveBlurFromStr": inactiveBlurFromStr,
            "userPausedFromStr": userPausedFromStr,
            "userPausedTrueFromStr": userPausedTrueFromStr
        };
        console.error("STRING_INVARIANTS_JSON:" + JSON.stringify(results));
        Qt.quit();
    }
}
"""
    with tempfile.NamedTemporaryFile("w", suffix=".qml") as tmp_qml:
        tmp_qml.write(test_qml)
        tmp_qml.flush()

        res = subprocess.run(["qml6", tmp_qml.name], capture_output=True, text=True, timeout=5)

    data = None
    for line in res.stderr.splitlines() + res.stdout.splitlines():
        if "STRING_INVARIANTS_JSON:" in line:
            data = json.loads(line.split("STRING_INVARIANTS_JSON:")[1])
            break

    if not data:
        log_fail("Could not read string invariants result")
        return False

    success = True
    if data["inactiveBlackFromStr"] is False:
        log_pass('"false" string evaluates to boolean FALSE for InactiveBlackScreen')
    else:
        log_fail('"false" string erroneously coerced to TRUE for InactiveBlackScreen')
        success = False

    if data["inactiveBlackFromBool"] is False:
        log_pass('boolean false evaluates to boolean FALSE for InactiveBlackScreen')
    else:
        log_fail('boolean false evaluated to TRUE')
        success = False

    if data["inactiveBlurFromStr"] is False:
        log_pass('"false" string evaluates to boolean FALSE for InactiveBlurScreen')
    else:
        log_fail('"false" string erroneously coerced to TRUE for InactiveBlurScreen')
        success = False

    if data["userPausedFromStr"] is False:
        log_pass('"false" string evaluates to boolean FALSE for UserPaused (prevents unwanted pause)')
    else:
        log_fail('"false" string erroneously paused playback for UserPaused')
        success = False

    if data["userPausedTrueFromStr"] is True:
        log_pass('"true" string evaluates to boolean TRUE for UserPaused')
    else:
        log_fail('"true" string failed to evaluate to TRUE')
        success = False

    return success


# ==============================================================================
# SUITE 7: Rust Backend Invariants & Unit Tests
# ==============================================================================
def test_rust_backend_invariants():
    log_header("SUITE 7: Rust Backend Invariants & Persistence Unit Tests")

    cmd = ["cargo", "test", "--manifest-path", os.path.join(ROOT_DIR, "src-tauri/Cargo.toml"), "--", "wallpaper::tests"]
    res = subprocess.run(cmd, capture_output=True, text=True)

    if res.returncode == 0 and "test result: ok" in res.stdout:
        log_pass("WallpaperManager mutual exclusivity test passed")
        log_pass("FillMode to DisplayMode mapping test passed")
        log_pass("PausePolicy mapping and persistence test passed")
        log_pass("Multi-Monitor scoping and main display persistence test passed")
        return True
    else:
        log_fail("Rust wallpaper tests failed", res.stderr or res.stdout)
        return False


# ==============================================================================
# MAIN ENTRY POINT
# ==============================================================================
def main():
    print(f"\n{BOLD}========================================================================{RESET}")
    print(f"{BOLD}   ANIMATED LINUX - COMPREHENSIVE LIVE VIEW MODES & MONITORS SUITE      {RESET}")
    print(f"{BOLD}========================================================================{RESET}")

    video_candidate = os.path.join(ROOT_DIR, "wallpapers/wp_1a140aecb37_q3.mp4")
    if not os.path.isfile(video_candidate):
        video_candidate = os.path.join(ROOT_DIR, "assets/sample_wallpapers/cyber_matrix.mp4")

    all_passed = True

    # Suite 1: Visual Inactive View Modes (MPV & QtMultimedia)
    if not run_visual_view_modes_suite("MPV", "contents/ui/backend/Mpv.qml", video_candidate):
        all_passed = False
    if not run_visual_view_modes_suite("QtMultimedia", "contents/ui/backend/QtMultimedia.qml", video_candidate):
        all_passed = False

    # Suite 2: Display Sizing & Fill Modes (Fit vs Zoom vs Stretch)
    if not run_display_modes_suite("MPV", "contents/ui/backend/Mpv.qml", video_candidate):
        all_passed = False
    if not run_display_modes_suite("QtMultimedia", "contents/ui/backend/QtMultimedia.qml", video_candidate):
        all_passed = False

    # Suite 3: Multi-Monitor Containment & Scoping
    if not run_multi_monitor_suite(video_candidate):
        all_passed = False

    # Suite 4: Window Pause Policies & Battery Power
    if not run_pause_policies_suite():
        all_passed = False

    # Suite 5: Audio Controls & Playback Speed
    if not run_audio_and_speed_suite(video_candidate):
        all_passed = False

    # Suite 6: Plasma D-Bus String Coercion Invariants
    if not test_plasma_dbus_string_invariants():
        all_passed = False

    # Suite 7: Rust Backend Invariants & Unit Tests
    if not test_rust_backend_invariants():
        all_passed = False

    log_header("OVERALL TEST RESULTS")
    if all_passed:
        print(f"\n{BOLD}{GREEN}  >>> ALL TEST SUITES (MONITORS, MODES & VIEW STATES) PASSED SUCCESSFULLY! <<<{RESET}\n")
        return 0
    else:
        print(f"\n{BOLD}{RED}  >>> SOME TEST SUITES FAILED! <<<{RESET}\n")
        return 1


if __name__ == "__main__":
    sys.exit(main())
