# Run and test dyland

Use a terminal inside a GNOME session. An SSH session has no display, so the island cannot open there.

## 1. Install the libraries

The project does not build until these packages are present. WebKit and libsoup are the ones that usually stop the compile.

Fedora:

```bash
sudo dnf install -y \
  gcc gcc-c++ make pkgconf-pkg-config \
  webkit2gtk4.1-devel \
  gtk3-devel \
  libsoup3-devel \
  libayatana-appindicator-gtk3-devel
```

Debian or Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libsoup-3.0-dev \
  libayatana-appindicator3-dev
```

You also need Rust 1.78+ (`cargo` on your `PATH`) and Node.js 20+ (`npm`).

## 2. Build, install, and start

From the repository root, as your normal user:

```bash
make build-release
sudo make install
make enable
```

`make enable` does not use `sudo`. It starts the island in this session. The next login starts it on its own from `/etc/xdg/autostart/dyland.desktop`.

If the GNOME clock is still in the top bar, log out and back in once. GNOME keeps the previous shell extension until the shell restarts.

## 3. What you should see

- One pill at the top center of the primary screen, sitting above the top bar.
- The GNOME clock is hidden.
- Clicks on the empty desktop still reach the windows underneath. Clicks on the pill open the island.
- A tray icon with Restart and Quit.

## 4. Short checks

Second launch stays a single island and exits 0:

```bash
dyd-ui; echo $?
```

Quit stays stopped. Choose **Quit** in the tray. The island should not come back. Start it again with `make enable`.

A crash comes back. This kills the UI process. systemd should start it again within a few seconds:

```bash
systemctl --user kill -s KILL dyd-ui.service
systemctl --user is-active dyd-ui.service
```

`active` means it recovered. `systemctl --user stop dyd-ui.service` is a clean stop and should stay stopped.

Login. Log out and back in. The island should appear without running `make enable` again.

## 5. If it does not start

```bash
cat ~/.local/state/dyland/session.log
cat ~/.local/state/dyland/ui.log
journalctl --user -u dyd-ui.service -u dyd-daemon.service -n 80 --no-pager
gnome-extensions info dyland@dyland.app
```

The extension id is `dyland@dyland.app`. `State: ENABLED` and a session log with no error means the login hook ran.

Turn it off:

```bash
make disable
```

Optional compile check, before install:

```bash
cargo test -p dyd-ui-tauri --lib
npm run typecheck
```
