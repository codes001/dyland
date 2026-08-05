# User Guide — dyland

Welcome to **dyland**, the Dynamic Island for Linux desktops.

---

## 🚀 Overview & Gestures

The Dynamic Island sits at the top center of your screen, seamlessly morphing to display active notifications, media controls, and system status indicators.

### Interaction Gestures
- **Single Click / Tap**: Expands active notification, media player controls, or status detailed view.
- **Long Press**: Opens context menu (Settings, Do Not Disturb mode).
- **Swipe Down / Up**: Dismisses expanded view or collapses active pill.
- **Keyboard Navigation**:
  - `Space` / `Enter`: Expand active item.
  - `Escape`: Collapse expanded view / close settings.
  - `Left` / `Right` Arrow: Cycle through active activity queue.
  - `S`: Open/Close Settings panel.
  - `D`: Toggle Do Not Disturb mode.

---

## 🎵 Media Integration (MPRIS)
Automatically detects active MPRIS media players (Spotify, Firefox, VLC, Rhythmbox, Amberol):
- **Compact View**: Displays animated equalizer / album art & track title.
- **Expanded View**: Play/Pause, Next/Previous track, Seek bar, Volume slider, and Album Art display.

---

## 🔔 Notifications
Intercepts D-Bus Desktop Notifications:
- **Urgent Notifications**: High-priority alert styling.
- **Per-App Controls**: Configure allowed apps, silent notifications, or queue-only in Settings.
- **Do Not Disturb**: Mutes incoming notifications.

---

## ⚡ System Status Indicators
Displays compact status icons in the pill:
- **Battery**: Percentage & charge/discharge health.
- **Wi-Fi**: SSID, signal strength, & IP address.
- **Bluetooth**: Connected devices & power toggle.
- **Volume & Brightness**: Level sliders & mute toggle.

---

## ⚙️ Settings & Customization
Long-press the island or press `S` to open the Settings panel:
- **General**: Start on login preference.
- **Theme**: Select `Dark`, `Light`, or `Follow System` (`prefers-color-scheme`).
- **Features**: Toggle individual system indicators on or off.
- **Shortcuts**: Rebind keyboard shortcuts to custom hotkeys.
- **Accessibility**: Reduce Motion toggle, High Contrast Mode, and Font Scaling slider (75%–150%).
- **Plugins**: Manage WASM extensions.
