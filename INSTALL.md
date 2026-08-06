# Installation Guide — dyland

**dyland** is a Dynamic Island for Linux desktops built with Rust, Tauri, React, and WebAssembly.

---

## 📋 System Requirements
- **OS**: Ubuntu 24.04 LTS, Debian 12+, Arch Linux, or Fedora 40+
- **Desktop Environment**: GNOME 46+ (Wayland or X11)
- **Dependencies**: `libwebkit2gtk-4.1`, `libgtk-3-0`, `libayatana-appindicator3-1`, `upower`, `NetworkManager`, `bluez`

---

## 📦 Installation Options

### Option 1: Debian / Ubuntu Package (`.deb`)
Download the latest `.deb` release from [GitHub Releases](https://github.com/codes001/dyland/releases):

```bash
sudo dpkg -i dyland_0.1.0_amd64.deb
sudo apt-get install -f # Install missing system dependencies if required
```

Enable and start the user background daemon:
```bash
systemctl --user daemon-reload
systemctl --user enable --now dyd-daemon.service
```

---

### Option 2: Portable AppImage
Download the `.AppImage` binary and make it executable:

```bash
chmod +x dyland-0.1.0-x86_64.AppImage
./dyland-0.1.0-x86_64.AppImage
```

---

### Option 3: Building from Source

#### Prerequisites
- Rust 1.78+ toolchain: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js 20+: `nvm install 20`
- System libraries:
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev build-essential
  ```

#### Build & Install
```bash
git clone https://github.com/codes001/dyland.git
cd dyland
make build-release
sudo make install
```

Start the background service:
```bash
systemctl --user enable --now dyd-daemon.service
```

---

## 🗑️ Uninstallation

If installed via `.deb`:
```bash
sudo dpkg -r dyland
```

If installed via `make install`:
```bash
sudo make uninstall
```
