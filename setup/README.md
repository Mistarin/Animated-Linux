# Animated Linux Setup & Scripts

Utility scripts for building, packaging, containerizing, and launching Animated Linux.

---

## Available Scripts

| Script | Description | Usage |
| :--- | :--- | :--- |
| `launch.sh` | Launches Animated Linux (uses release binary if available, or `--dev` for live development). | `./setup/launch.sh` or `./setup/launch.sh --dev` |
| `build_image.sh` | Compiles web assets & Rust binary, creates distribution artifacts, and optionally builds container images. | `./setup/build_image.sh` |
| `install-steam-shortcut.sh` | Registers Animated Linux into local Steam user libraries with custom banners, capsules, and logos. | `./setup/install-steam-shortcut.sh` |
| `Dockerfile` | Multi-stage container recipe for building and running Animated Linux in a reproducible Linux environment. | `docker build -f setup/Dockerfile -t animated-linux:latest .` |

---

## Examples

### 1. Launch Application
```bash
# Production launch:
./setup/launch.sh

# Live development mode (HMR):
./setup/launch.sh --dev
```

### 2. Build Release Artifacts
```bash
# Build native binary into dist/:
./setup/build_image.sh

# Build container image:
./setup/build_image.sh --docker
```

### 3. Add to Steam
```bash
./setup/install-steam-shortcut.sh
```
Restart Steam to see Animated Linux in your Steam Library.
