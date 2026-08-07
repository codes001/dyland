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
- **System Dependencies**:
  - `libwebkit2gtk-4.1-dev` (or `libwebkit2gtk-4.0`)
  - `libgtk-3-dev`
  - `libayatana-appindicator3-dev`
  - `upower`, `NetworkManager`, `bluez`
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
sudo make install       # copy binaries + autostart services
make enable             # NO sudo — register autostart for your login
```

Done. The island appears at the top of your screen. Remove it any time:

```bash
make disable            # stop it and remove autostart
sudo make uninstall     # remove all files
```

See [INSTALL.md](INSTALL.md) for details, .deb/AppImage options and troubleshooting.

---

### Option 3: Pre-packaged Distribution (`.deb` or AppImage)

#### Installing via `.deb` package:
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

- **Buy Me a Coffee**: [buymeacoffee.com](https://www.buymeacoffee.com)
- **GitHub Sponsors**: [github.com/sponsors/codes001](https://github.com/sponsors/codes001)
- **Ko-fi**: Support via [ko-fi.com](https://ko-fi.com)
- **Star the Repo**: Give us a star on GitHub to spread the word.

Thank you for supporting open-source software!

---

## License

This project is licensed under the [MIT License](LICENSE).
