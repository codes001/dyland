# Installation Guide — dyland

**dyland** is a Dynamic Island for Linux desktops built with Rust, Tauri, React, and WebAssembly.

After installing, dyland is a permanent part of your desktop: it auto-starts at
every login, survives reboots, comes back automatically if it ever crashes, and
only goes away when you uninstall it.

---

## System Requirements
- **OS**: Ubuntu 24.04 LTS, Debian 12+, Arch Linux, or Fedora 40+
- **Desktop Environment**: GNOME 46+ (Wayland or X11)
- **Dependencies**: `libwebkit2gtk-4.1`, `libgtk-3-0`, `libayatana-appindicator3-1`, `upower`, `NetworkManager`, `bluez`

---

## Installation Options

### Option 1: Building from Source (recommended)

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
sudo make install        # copies binaries + autostart services
make enable              # NO sudo — registers autostart for YOUR login
```

That's it. The island appears at the top of your screen within a few seconds.

#### What `make enable` does
- Starts the daemon (`dyd-daemon.service`) and the island window (`dyd-ui.service`) now
- Registers both as user systemd services started at every login
- Both auto-restart if they crash (`Restart=on-failure` for both)
- The island window cannot be closed by accident (closing it is ignored)

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

Enable autostart for your login (run as your normal user):
```bash
systemctl --user daemon-reload
systemctl --user enable --now dyd-daemon.service dyd-ui.service
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
| Island not visible after login | `systemctl --user status dyd-ui.service` — check it started after the graphical session |
| Island visible but blank/connecting | `systemctl --user status dyd-daemon.service`, check log with `journalctl --user -u dyd-daemon -f` |
| Services stopped at login | You ran `make enable` from a non-graphical session (e.g. ssh). Run it from your desktop terminal |
| Uninstall didn't remove the island | `make disable` first, then `sudo make uninstall` |
