# dyland

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/tauri-v1-24C8D8.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/react-18-61DAFB.svg)](https://react.dev/)

**dyland** (`dyd`) is a Dynamic Island desktop overlay for Linux environments (GNOME / Wayland / X11). Built using a decoupled architecture with **Rust** for background system hooks and **Tauri + React + Framer Motion** for spring animations, dyland brings notifications, media controls, status indicators, and extensible WASM plugins to your Linux desktop.

---

## Features

- **Notification Interception**: Live D-Bus notification capture presented directly within the interactive island UI.
- **Media Integration**: Full MPRIS support with album art rendering, player selection, and playback controls (Play, Pause, Skip, Prev).
- **System Status Indicators**: Real-time D-Bus monitoring for Battery status, Wi-Fi connectivity, Bluetooth state, Volume adjustment, and Screen Brightness.
- **WASM Plugin Engine**: Sandboxed plugin runtime (`wasmtime`) allowing developers to create custom island modules in Rust using `dyd-plugin-sdk`.
- **Theme & Customization System**: Support for Dark, Light, and System themes, custom keyboard shortcuts, reduced motion mode, and per-app notification preferences.
- **Performant & Isolated**: Decoupled dual-process architecture separating background system interactions (`dyd-daemon`) from the UI overlay (`dyd-ui`).

---

## Architecture Overview

```
                  +-----------------------------------+
                  |            GNOME Desktop          |
                  +-----------------+-----------------+
                                    | D-Bus Signals
                                    v
+-------------------------------------------------------------------+
|                            dyd-daemon                             |
|  (Rust Core Service — Lifecycle, D-Bus Listeners, WASM Runtime)   |
|                                                                   |
|  +--------------------+  +------------------+  +---------------+  |
|  | D-Bus Sources      |  | Activity Queue   |  | ConfigStore   |  |
|  | (MPRIS, Notifs...) |  | (Priority Heap)  |  | (config.json) |  |
|  +---------+----------+  +--------+---------+  +-------+-------+  |
|            |                      |                    |          |
|            +----------------------+--------------------+          |
|                                   v                               |
|                        IPC Server (WebSocket)                     |
+-----------------------------------|-------------------------------+
                                    | JSON-RPC Envelopes
                                    v
+-------------------------------------------------------------------+
|                              dyd-ui                               |
|   (Tauri Transparent Overlay Window — React, Motion, Zustand)     |
|                                                                   |
|  +--------------------+  +------------------+  +---------------+  |
|  | Pill / State Engine|  | Expanded Views   |  | Theme System  |  |
|  +--------------------+  +------------------+  +---------------+  |
+-------------------------------------------------------------------+
```

Detailed architectural information can be found in [`ARCHITECTURE.md`](ARCHITECTURE.md).

---

## Prerequisites

Before running or building **dyland**, ensure your system meets the following requirements:

- **Operating System**: Linux (Ubuntu 24.04+, Debian 12+, Arch Linux, Fedora 40+)
- **Desktop Environment**: GNOME 46+ (Wayland or X11 recommended)
- **System Dependencies** (Fedora):

  ```bash
  sudo dnf install -y \
    gcc gcc-c++ make pkgconf-pkg-config \
    webkit2gtk4.1-devel \
    gtk3-devel \
    libsoup3-devel \
    libayatana-appindicator-gtk3-devel \
    upower NetworkManager bluez
  ```

  Debian and Ubuntu use the same libraries under different names: `build-essential`, `pkg-config`, `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev`, `libayatana-appindicator3-dev`, `upower`, `network-manager`, and `bluez`.

- **Toolchains** (for running from source):
  - **Rust**: `1.78+` (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
  - **Node.js**: `20+` (`npm` / `nvm`)

---

## How to Run dyland

### Option 1: Quick Development Run

To launch `dyland` in local development mode (runs both the Rust backend daemon and Tauri UI overlay simultaneously):

```bash
# Clone the repository
git clone https://github.com/codes001/dyland.git
cd dyland

# Install frontend dependencies
npm install

# Start both daemon and UI concurrently
npm run dev
```

Alternatively, you can run the components in separate terminals:

```bash
# Terminal 1: Run the Rust daemon
cargo run -p dyd-daemon

# Terminal 2: Run the Tauri React UI
npm run dev -w dyd-ui
```

---

### Option 2: Build, Install & Enable (permanent — recommended)

dyland becomes a permanent part of your desktop: it auto-starts at every login,
survives reboots, auto-restarts if it crashes, and stays until you uninstall it.

```bash
make build-release      # build daemon + island binaries
sudo make install       # binaries, XDG autostart, systemd units, GNOME extension
make enable             # NO sudo — start it in this session (next login does this itself)
```

Done. After the next login the island is at the top of the screen, above the
GNOME clock. Remove it any time:

```bash
make disable            # stop it and remove autostart
sudo make uninstall     # remove all files
```

See [INSTALL.md](INSTALL.md) for details, .deb/AppImage options and troubleshooting.

---

### Option 3: Pre-packaged Distribution (`.deb` or AppImage)

#### Installing via `.deb` package (Debian and Ubuntu only):
```bash
sudo dpkg -i dist/dyland_0.1.0_amd64.deb
sudo apt-get install -f # Resolve any missing dependencies

# Start daemon service
systemctl --user daemon-reload
systemctl --user enable --now dyd-daemon.service
```

#### Running via Portable AppImage:
```bash
make appimage
chmod +x dist/dyland-0.1.0-x86_64.AppImage
./dist/dyland-0.1.0-x86_64.AppImage
```

For more detailed setup options, refer to [`INSTALL.md`](INSTALL.md).

---

## Updating dyland

Replace `<version>` with the latest release version (e.g. `0.1.1`).

**Via `.deb`:**
```bash
sudo dpkg -i dyland_0.1.1_amd64.deb && systemctl --user restart dyd-daemon dyd-ui
```

**Via source (`make install`):**
```bash
git pull && make build-release && sudo make install && systemctl --user restart dyd-daemon dyd-ui
```

**Via AppImage:**
```bash
chmod +x dyland-<version>-x86_64.AppImage && ./dyland-<version>-x86_64.AppImage
```

---

## Project Commands

| Command | Action |
|:--- |:--- |
| `npm run dev` | Runs daemon and UI concurrently in development mode |
| `npm run build` | Builds Rust daemon and Tauri UI |
| `npm run lint` | Runs `clippy` and ESLint checks |
| `npm run format` | Formats Rust (`cargo fmt`) and TypeScript (`prettier`) |
| `npm run typecheck` | Validates TypeScript types in `dyd-ui` |
| `make deb` | Packages project into a Debian `.deb` installer |
| `make appimage` | Builds a standalone portable AppImage |

---

## Support the Developer

If you find **dyland** helpful and enjoy having a Dynamic Island experience on your Linux desktop, please consider supporting development.

### Ways to Support:

- **Buy Me a Coffee**: [buymeamalt.com](https://buymeamalt.com/@codes001)
- **GitHub Sponsors**: [github.com/sponsors/codes001](https://github.com/sponsors/codes001)
- **Star the Repo**: Give us a star on GitHub to spread the word.

Thank you for supporting open-source software!

---

## License

This project is licensed under the [MIT License](LICENSE).
