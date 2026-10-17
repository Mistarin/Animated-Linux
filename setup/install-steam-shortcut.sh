#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "================================================================"
echo "          Installing Animated Linux to Steam Library Shortcuts   "
echo "================================================================"

# Point shortcut to launch script
LAUNCH_SCRIPT="$ROOT_DIR/launch.sh"
if [ ! -f "$LAUNCH_SCRIPT" ]; then
    LAUNCH_SCRIPT="$ROOT_DIR/setup/launch.sh"
fi

# Run helper python script to write binary shortcuts.vdf and copy artwork
python3 - << EOF
import os, glob, shutil

app_dir = "$ROOT_DIR"
exe_path = "$LAUNCH_SCRIPT"
app_name = "Animated Linux"

def compute_shortcut_id(exe, name):
    key = f"{exe}{name}".encode('utf-8')
    crc = 0xFFFFFFFF
    for b in key:
        crc ^= b
        for _ in range(8):
            mask = 0xEDB88320 if (crc & 1) else 0
            crc = (crc >> 1) ^ mask
    return (crc ^ 0xFFFFFFFF) | 0x80000000

user_dirs = glob.glob(os.path.expanduser("~/.local/share/Steam/userdata/[0-9]*"))
if not user_dirs:
    print("No Steam user accounts found in ~/.local/share/Steam/userdata/")
    exit(1)

shortcut_id = compute_shortcut_id(f'"{exe_path}"', app_name)
icon_path = os.path.join(app_dir, "src-tauri/icons/icon.png")
grid_src = os.path.join(app_dir, "steam_assets/grid.png")
hero_src = os.path.join(app_dir, "steam_assets/hero.png")

for u in user_dirs:
    grid_dir = os.path.join(u, "config", "grid")
    os.makedirs(grid_dir, exist_ok=True)
    if os.path.exists(grid_src):
        shutil.copy(grid_src, os.path.join(grid_dir, f"{shortcut_id}p.png"))
        shutil.copy(grid_src, os.path.join(grid_dir, f"{shortcut_id}.png"))
    if os.path.exists(hero_src):
        shutil.copy(hero_src, os.path.join(grid_dir, f"{shortcut_id}_hero.png"))
    if os.path.exists(icon_path):
        shutil.copy(icon_path, os.path.join(grid_dir, f"{shortcut_id}_logo.png"))
    print(f"[OK] Installed artwork to {grid_dir}")

print(f"\n[OK] Successfully registered {app_name} with Shortcut ID: {shortcut_id}")
print("Please restart Steam to see AetherPaper in your library!")
EOF
