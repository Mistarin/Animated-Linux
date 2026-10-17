# Animated Linux Setup & Scripts

Utility scripts for launching, developing, and managing Animated Linux.

---

## Available Scripts

| Script | Description | Usage |
| :--- | :--- | :--- |
| `launch.sh` | Launches Animated Linux executable through the launch script with proper environment fixes. Supports running directly from source (`--dev`) or installing the desktop entry (`--install-desktop`). | `./launch.sh` |
| `install-steam-shortcut.sh` | Registers Animated Linux into local Steam user libraries, routing through `launch.sh`. | `./setup/install-steam-shortcut.sh` |
| `build_image.sh` | Compiles the single standalone executable (`animated-linux-x86_64`) into `dist/` without bundles or packages. | `./setup/build_image.sh` |

---

## Examples

### 1. Launch Application
```bash
# Launch native executable:
./launch.sh

# Run directly from source with live reload (HMR):
./launch.sh --dev
```

### 2. Add Desktop Menu Entry
```bash
# Registers Animated Linux in the desktop application menu pointing to launch.sh:
./launch.sh --install-desktop
```

### 3. Build / Recompile Native Executable
```bash
# Compile standalone native executable:
./launch.sh --build
```

### 4. Add to Steam
```bash
./setup/install-steam-shortcut.sh
```
Restart Steam to see Animated Linux in your Steam Library.
