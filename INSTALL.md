# Installation Guide — dyland

**dyland** is a Dynamic Island for Linux desktops built with Rust, Tauri, React, and WebAssembly.

After installing, dyland is a permanent part of your desktop: the next graphical
login starts it, it comes back if it crashes, and it stays until you disable
or uninstall it. On GNOME the shell extension hides the clock and stacks the
island above the top bar. See [docs/desktop-integration.md](docs/desktop-integration.md).

---

## System Requirements
- **OS**: Ubuntu 24.04 LTS, Debian 12+, Arch Linux, or Fedora 40+
- **Desktop Environment**: GNOME 46+ (Wayland or X11)
- **Dependencies**: WebKitGTK 4.1, GTK 3, libayatana-appindicator, `upower`, NetworkManager, `bluez`

---

## Installation Options

### Option 1: Building from Source (recommended)

#### Prerequisites
- Rust 1.78+ toolchain: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js 20+: `nvm install 20`
- System libraries. On Fedora:

  ```bash
  sudo dnf install -y \
    gcc gcc-c++ make pkgconf-pkg-config \
    webkit2gtk4.1-devel \
    gtk3-devel \
    libsoup3-devel \
    libayatana-appindicator-gtk3-devel
  ```

  On Debian or Ubuntu:

  ```bash
  sudo apt-get install -y \
    build-essential pkg-config \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libsoup-3.0-dev \
    libayatana-appindicator3-dev
  ```

#### Build & Install
```bash
git clone https://github.com/codes001/dyland.git
cd dyland
make build-release
sudo make install        # binaries, XDG autostart, systemd units, shell extension
make enable              # optional, NO sudo — start it in this session now
```

The next login starts the island on its own. `make enable` is only how you
start it without logging out. The first time the extension loads, log out and
back in once if the clock is still visible.

#### What startup does
- `/etc/xdg/autostart/dyland.desktop` runs `dyland-session` after graphical login
- That enables the GNOME Shell extension and the user services
- `dyd-daemon` and `dyd-ui` restart on crash (`Restart=on-failure`)
- A second launch talks to the running island instead of opening another one
- Tray **Quit** or `make disable` stays off. Closing the window does not
- Logs: `~/.local/state/dyland/session.log`, `ui.log`, and `journalctl --user -u dyd-ui`

#### Stopping it without uninstalling
The app shows a tray icon (Restart / Quit). Choose **Quit** to stop the island
until your next login (or `systemctl --user start dyd-ui.service` to bring it
back immediately). Crashes still restart automatically — only an explicit Quit
stops it.

---

### Option 2: Debian / Ubuntu Package (`.deb`)
Download the latest `.deb` release from [GitHub Releases](https://github.com/codes001/dyland/releases):

```bash
sudo dpkg -i dyland_0.1.0_amd64.deb
sudo apt-get install -f # Install missing system dependencies if required
```

The package registers `/etc/xdg/autostart/dyland.desktop`. The next login
starts the island and enables the user services. To start it immediately:

```bash
systemctl --user daemon-reload
systemctl --user import-environment DISPLAY WAYLAND_DISPLAY XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS
systemctl --user reenable dyd-daemon.service dyd-ui.service
systemctl --user start dyd-daemon.service dyd-ui.service
gnome-extensions enable dyland@dyland.app
```

---

### Option 3: Portable AppImage
Download the `.AppImage` binary and make it executable:

```bash
chmod +x dyland-0.1.0-x86_64.AppImage
./dyland-0.1.0-x86_64.AppImage
```

AppImage does not register autostart — to make it start at login, add it to
`~/.config/autostart/` via your desktop's "Startup Applications" settings.

---

## Uninstallation

```bash
make disable        # stop services and remove autostart registration
sudo make uninstall # remove binaries, services and desktop entry
```

If installed via `.deb`:
```bash
systemctl --user disable --now dyd-daemon.service dyd-ui.service
sudo dpkg -r dyland
```

---

## Development Mode (not installed)

Run the daemon and island manually, one terminal each:

```bash
# Terminal 1 — daemon
cargo run -p dyd-daemon

# Terminal 2 — island window
cd dyd-ui && npm run tauri dev
```

Note: `npm run dev` alone only runs the daemon + a browser preview of the UI —
the actual island is the Tauri window above.

## Troubleshooting

| Symptom | Fix |
|---|---|
| Island not visible after login | `systemctl --user status dyd-ui.service` and `~/.local/state/dyland/session.log` |
| Clock still visible over the island | `gnome-extensions info dyland@dyland.app` — log out once so the extension loads |
| Island visible but blank/connecting | `systemctl --user status dyd-daemon.service`, `journalctl --user -u dyd-daemon -f` |
| Second icon after launching again | The running instance should come forward. Check `~/.local/state/dyland/ui.log` |
| Services stopped at login | Run `make enable` from a desktop terminal, not ssh |
| Uninstall didn't remove the island | `make disable` first, then `sudo make uninstall` |
