# Desktop integration

This is the production overlay and login design for dyland on GNOME. It is the
reason the island can occupy the top-center of the screen, and the reason it
comes back after login and after a crash.

## Why the old window could not cover the clock

The island was a normal Tauri toplevel with `alwaysOnTop`. On GNOME that flag
only reorders client windows inside Mutter's window group. The top bar is not a
window. It is shell chrome drawn in the UI group, which sits above every
client surface, including windows marked above. Raising the z-index again
cannot change that.

`setPosition(0, 0)` does not fix it either. Mutter reserves a strut for the
panel and clamps client coordinates to the work area under the bar. On Wayland,
`primary_monitor()` is also empty, so the previous placement code guessed from
`window.screen` and device pixels and then fought the compositor with a
negative Y. The clock stayed in the panel, and the island sat underneath it.

`skipTaskbar` was false, so the same window also appeared in the taskbar and
in Alt+Tab.

## Why layer-shell is not the GNOME fix

`zwlr_layer_shell_v1` is the correct protocol for a desktop overlay on
wlroots compositors (Sway, Hyprland) and on KWin. GTK4 Layer Shell is a client
library for that protocol. It does nothing on GNOME: Mutter, through the 51
series, does not advertise the protocol. Shipping a layer-shell dependency
would fail closed on the desktop this project targets and would not hide the
clock.

XDG window types (`DOCK`, `UTILITY`) and "always on top" are client hints.
Mutter does not promote those hints into the shell layer.

## Architecture

```
graphical login
    -> /etc/xdg/autostart/dyland.desktop
        -> dyland-session
            -> import Wayland/X11 environment into systemd --user
            -> enable dyland@dyland.app
            -> systemd user units (Restart=on-failure)
                -> dyd-daemon
                -> dyd-ui
                    -> GNOME Shell extension stacks the surface
                       above the panel and hides the clock
```

The Tauri window remains the island. The shell extension does not draw a
second one. Its only job is compositor integration:

- find the surface whose GTK role is `dyland-overlay` (app id `app.dyland`)
- move that actor from the window group into `Main.layoutManager.uiGroup`,
  above `panelBox`
- place it at the top center of `primaryMonitor`
- hide `statusArea.dateMenu` so the clock cannot show through the transparent
  pixels
- repeat on monitor changes, and once a second in case Mutter puts the
  surface back in the window group after a configure
- on overview or the lock screen, return the surface to the window group and
  show the clock again
- on disable or process exit, restore the clock

Monitor geometry comes from the shell in logical pixels, so resolution,
scale, and which output is primary stay correct without a device-pixel
guess. The window itself is a fixed 440px-wide column. Compact mode is 96px
tall. Expanded mode is 760px tall. The shell recenters that rectangle on the
primary output, including outputs whose origin is not `(0, 0)`.

The window is an undecorated utility surface: no taskbar entry, no pager
entry, not focused on map, and stuck to all workspaces. Those hints are what
X11 and non-GNOME sessions honor. They are not a substitute for the extension
on GNOME.

Input uses the GDK input region, fed by the island's on-screen rectangles.
Empty overlay pixels do not take clicks. The pill, the expanded card, and the
context menu do. Fullscreen hides the surface so the app underneath receives
input, and the extension puts the clock back while the surface is unmapped.

## Login, one instance, crashes

XDG autostart is the login hook. `make install` writes
`/etc/xdg/autostart/dyland.desktop`. A graphical login runs
`dyland-session` about a second after the session is up. No per-user enable
step is required for the next login.

`dyland-session` then:

1. Refuses to start when neither `WAYLAND_DISPLAY` nor `DISPLAY` is set.
2. Installs and enables the shell extension for the logged-in user.
3. Imports the session environment into `systemd --user` so the GUI units can
   see the display.
4. Re-enables and starts `dyd-daemon.service` and `dyd-ui.service`.

The units are `WantedBy=graphical-session.target`, `Restart=on-failure`,
`RestartSec=2s`, with a start limit of 10 failures in 2 minutes. A clean exit
(code 0) stays stopped. A crash restarts. `SIGTERM` and `SIGINT` exit 0 so
logout and an explicit Quit are not treated as crashes. Closing the window
from the compositor is ignored. Quit is the tray menu.

If the systemd units do not stay active, the session script stops the UI unit
and supervises `dyd-ui` itself with the same rule: restart on non-zero, stop
on zero. Failures are appended to
`$XDG_STATE_HOME/dyland/session.log` and the UI also writes
`$XDG_STATE_HOME/dyland/ui.log` plus stderr (so `journalctl --user -u dyd-ui`
has them when systemd is the supervisor).

`make disable` disables the user units, writes
`~/.config/autostart/dyland.desktop` with `Hidden=true` (this overrides the
system autostart entry), and disables the shell extension so the clock
returns. `make enable` removes that override, re-enables the units, and
enables the extension.

## Single instance

`dyd-ui` takes an exclusive `flock` on `$XDG_RUNTIME_DIR/dyland/ui.lock`
before it creates a window. The holder listens on
`$XDG_RUNTIME_DIR/dyland/ui.sock`. A second launch connects, sends `raise`,
and exits 0. The running process shows and recenters the existing surface.
It does not activate the window, so it does not steal key focus.

The lock is released when the process exits, including a crash, because the
kernel drops `flock` with the process. A stale socket is removed by the next
holder before it binds. A clean shutdown removes the socket as well.

After the window is placed, the process writes
`$XDG_STATE_HOME/dyland/ui-health.json` with pid, version, monitor source,
and overlay size. `status` is `degraded` when GTK cannot see a monitor yet.

The daemon keeps its own single instance by binding `/tmp/dyd-daemon.sock`.
The UI does not start a second daemon when systemd already did.

## What still depends on the session

The extension loads when GNOME enables it. The first login after install does
that from `dyland-session`. A shell that was already running with an older
copy of the extension keeps the old code until that shell restarts. Log out
and back in once after upgrading the extension.

Without the extension, the window is still top-center of the monitor GTK
calls primary (or the output nearest the origin on Wayland), but it cannot
paint over the GNOME panel. That is a Mutter limitation, not a placement bug.
