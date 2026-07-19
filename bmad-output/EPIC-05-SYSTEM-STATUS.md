# Epic 5: System Status Indicators

## Objective
Monitor and display system status information (battery, WiFi, Bluetooth, volume, brightness) in the Dynamic Island. Status icons appear compactly in the pill when space permits, and provide quick controls in the expanded view.

## Business Value
System status is always relevant to users. Quick access to volume/brightness sliders and connectivity status eliminates the need to open system settings for common adjustments.

## Success Criteria
- Battery percentage + charging status shown in pill
- WiFi SSID + signal strength shown in pill
- Bluetooth status + connected device count shown in pill
- Volume slider works in expanded view
- Brightness slider works in expanded view
- All indicators update within 500ms of system change
- Indicators are hidden when not relevant (e.g., desktop without battery)

## Risks
- UPower, NetworkManager, and BlueZ D-Bus APIs are distro/version-specific
- Brightness control requires raw D-Bus calls to `org.gnome.SettingsDaemon.Power.Screen` or `org.freedesktop.UPower`
- PipeWire/WirePlumber for volume has a different API than PulseAudio
- Permissions: some D-Bus calls may be restricted by polkit

## Dependencies
- Epic 1 (IPC, daemon)
- Epic 2 (pill rendering, compact layout)

## Complexity
⭐⭐⭐ — Moderate. Multiple D-Bus interfaces to learn and handle, but each is individually straightforward.

---

## Feature 5.1: Battery Monitoring

**Description:** Monitor battery status via UPower D-Bus service (`org.freedesktop.UPower`). Display battery percentage, charging state, and estimated time remaining in the pill and expanded view.

**Acceptance Criteria:** Battery percentage shows in pill (compact icon + %). Charging state indicated by icon (bolt). Expanded view shows percentage, time remaining, and battery health (if available). Desktop (no battery) hides the indicator. Updates on UPower signals.

**Dependencies:** 1.2

**Notes:** Use UPower's `PropertiesChanged` signal for real-time updates. Do NOT poll.

### User Stories

#### US-5.1.1 — UPower Signal Subscription
**As a** system
**I want** to receive battery status changes from UPower
**so that** the Dynamic Island reflects current battery state.

**Acceptance Criteria:**
- Connect to UPower via D-Bus: `org.freedesktop.UPower`
- Subscribe to `PropertiesChanged` on the primary battery device (`/org/freedesktop/UPower/devices/DisplayDevice`)
- Capture: `Percentage`, `State` (charging/discharging/full), `TimeToEmpty`, `TimeToFull`, `IsPresent`
- Changes are emitted to UI via IPC within 200ms

**Edge Cases:**
- No battery present (desktop) → "IsPresent: false" → hide battery indicator
- Battery removed while system is running → signal fires, UI hides indicator
- Multiple batteries (some laptops) → use DisplayDevice which aggregates

**Technical Notes:**
- UPower device path: find via `EnumerateDevices` on `org.freedesktop.UPower`, filter by `type == 2` (battery)
- Struct:
  ```rust
  struct BatteryState {
      percentage: f64,
      state: BatteryState,
      time_to_empty: i64,  // seconds
      time_to_full: i64,   // seconds
      is_present: bool,
  }
  ```
- State enum: 0=unknown, 1=charging, 2=discharging, 3=empty, 4=fully charged, 5=pending charge, 6=pending discharge

**Priority:** P0

#### US-5.1.2 — Battery Pill Indicator
**As an** end user
**I want** to see my battery percentage in the Dynamic Island pill
**so that** I can check it at a glance.

**Acceptance Criteria:**
- Pill shows: [battery icon] + [percentage text] (e.g., "85%")
- Icon changes based on state:
  - Charging: battery icon + lightning bolt
  - Discharging: standard battery icon
  - Full: battery icon with checkmark (or "FULL" text)
  - Low (<20%): red battery icon
  - Critical (<10%): red + pulsing icon
- Compact width: icon + 3-4 digit text = ~60px total
- Multiple indicators in pill (battery + WiFi + etc.) show in a row, truncated if space is limited

**Edge Cases:**
- Battery removal/insertion → icon appears/disappears with animation
- Very low percentages (1-9%) → show with one decimal? No, just "5%"

**Technical Notes:**
- Component: `<BatteryIndicator>` uses SVG icons for each state
- Colors: normal (white), charging (yellow/green using CSS class), low (red)
- Icon set: closed (discharging), open + bolt (charging), full (full)
- Use `useMemo` to avoid re-rendering on unchanged values

**Priority:** P0

#### US-5.1.3 — Battery Expanded View
**As an** end user
**I want** to expand the battery indicator to see more details
**so that** I can see time remaining and battery health.

**Acceptance Criteria:**
- Expanded view shows:
  - Large battery icon (charging/discharging animation)
  - Percentage (large text)
  - Time remaining: "2h 15m remaining" or "1h 30m until full"
  - Battery capacity/health (if available from UPower)
  - Power source: "AC Adapter" or "Battery"
- Layout matches Apple's battery view aesthetics

**Edge Cases:**
- Time not available (calculating) → show "Estimating…"
- Battery full → show "Fully Charged"
- No battery → show nothing (indicator hidden)

**Technical Notes:**
- Component: `<BatteryExpandedView>`
- Time formatting: seconds → "Xh Ym" or "Xm" for under 1 hour
- Battery health: `EnergyFullDesign` vs `EnergyFull` ratio

**Priority:** P1

---

## Feature 5.2: WiFi Status

**Description:** Monitor WiFi connection status via NetworkManager D-Bus service (`org.freedesktop.NetworkManager`). Display connected SSID and signal strength in the pill.

**Acceptance Criteria:** Connected SSID shown in pill (truncated). Signal strength shown as bars. Disconnected state shows disconnected icon. Expanded view shows network details and quick toggle.

**Dependencies:** 1.2

**Notes:** NetworkManager emits `PropertiesChanged` on the active connection. Monitor access point for signal strength changes.

### User Stories

#### US-5.2.1 — NetworkManager Signal Subscription
**As a** system
**I want** to monitor WiFi state changes via NetworkManager
**so that** the Dynamic Island reflects current WiFi status.

**Acceptance Criteria:**
- Connect to NetworkManager via D-Bus: `org.freedesktop.NetworkManager`
- Track primary (active) WiFi connection
- Subscribe to `PropertiesChanged` on the active connection and its access point
- Capture: `Ssid`, `Strength` (0-100), `ConnectionState`, `Ip4Config`
- Changes emitted to UI via IPC within 500ms

**Edge Cases:**
- No WiFi interface (desktop with Ethernet only) → WiFi indicator hidden
- WiFi turned off → show disabled icon
- Connecting → show animated connecting indicator
- Airplane mode → show airplane icon

**Technical Notes:**
- Active connection: `org.freedesktop.NetworkManager` `/org/freedesktop/NetworkManager` property `PrimaryConnection` or `ActiveConnections`
- Access point properties via `/org/freedesktop/NetworkManager/AccessPoint/{n}`
- Struct:
  ```rust
  struct WifiState {
      ssid: Option<String>,
      strength: u8,       // 0-100
      state: WifiState,   // connected, connecting, disconnected, disabled
      ip_address: Option<String>,
  }
  ```

**Priority:** P1

#### US-5.2.2 — WiFi Pill Indicator
**As an** end user
**I want** to see my WiFi status in the Dynamic Island pill
**so that** I can check connectivity at a glance.

**Acceptance Criteria:**
- Pill shows: [WiFi icon (bars)] + [SSID or "WiFi"] (truncated to ~12 chars)
- Signal strength icon (0-3 bars):
  - 0-25%: 1 bar
  - 26-50%: 2 bars
  - 51-75%: 3 bars
  - 76-100%: 4 bars
- Disconnected: icon with slash (or X)
- Connecting: animated dots
- No WiFi hardware: hidden

**Edge Cases:**
- Hidden SSID → show "Hidden Network"
- SSID with non-ASCII characters → display as-is (UTF-8)
- Ethernet active with WiFi off → show Ethernet icon (optional)

**Technical Notes:**
- SVG icons: stacked bars (matching Apple's signal bars)
- Component: `<WifiIndicator>` with signal bars as CSS or SVG
- SSID text: `text-overflow: ellipsis; max-width: 80px;`

**Priority:** P1

#### US-5.2.3 — WiFi Expanded View
**As an** end user
**I want** to expand the WiFi indicator to see network details
**so that** I can view my IP address and signal strength.

**Acceptance Criteria:**
- Expanded view shows:
  - Network name (full SSID)
  - Signal strength bar (visual, 0-100%)
  - IP address
  - Frequency band (2.4GHz / 5GHz)
  - Security type (WPA2, WPA3, etc.)
- No controls (network switching is outside scope for now)

**Edge Cases:**
- No connection → show "No WiFi Connection"

**Technical Notes:**
- Component: `<WifiExpandedView>`
- Frequency: from access point `Frequency` property (< 5000 = 2.4GHz, >= 5000 = 5GHz)
- Security: from access point `WpaFlags` and `RsnFlags`

**Priority:** P2

---

## Feature 5.3: Bluetooth Status

**Description:** Monitor Bluetooth status via BlueZ D-Bus service (`org.bluez`). Display Bluetooth on/off state and connected device count in the pill.

**Acceptance Criteria:** Bluetooth icon shows in pill when on. Connected device count badge. Expanded view shows connected devices list. Pairing and connecting toggles are visible but not actionable (view-only for now).

**Dependencies:** 1.2

**Notes:** BlueZ D-Bus API is complex but well-documented. Use `PropertiesChanged` on adapter and device objects.

### User Stories

#### US-5.3.1 — BlueZ Signal Subscription
**As a** system
**I want** to monitor Bluetooth state changes via BlueZ
**so that** the Dynamic Island reflects Bluetooth status.

**Acceptance Criteria:**
- Connect to BlueZ via D-Bus: `org.bluez`
- Monitor adapter `/org/bluez/hci0` for `Powered` and `Discoverable` properties
- Monitor connected devices via `/org/bluez/hci0/dev_*`
- Capture: `Powered` bool, `ConnectedDevices` count + names
- Changes emitted to UI via IPC within 500ms

**Edge Cases:**
- No Bluetooth hardware → indicator hidden
- Bluetooth adapter not present → try alternative path
- Device connects/disconnects → update count

**Technical Notes:**
- Adapter property: `org.bluez.Adapter1.Powered`
- Connected devices: enumerate `/org/bluez/hci0/dev_*`, check `org.bluez.Device1.Connected`
- Struct:
  ```rust
  struct BluetoothState {
      powered: bool,
      connected_devices: Vec<BluetoothDevice>,
  }
  ```

**Priority:** P2

#### US-5.3.2 — Bluetooth Pill & Expanded View
**As an** end user
**I want** to see Bluetooth status in the Dynamic Island
**so that** I know if Bluetooth is active and what is connected.

**Acceptance Criteria:**
- Pill shows: [Bluetooth icon] + [device count if > 0]
- Icon is solid when on, outlined when off
- Expanded view shows:
  - Bluetooth toggle (on/off switch)
  - List of connected devices (name + device type icon)
  - "No devices connected" when empty
- Toggle is interactive (turn BT on/off)

**Edge Cases:**
- Bluetooth off → icon dimmed, no count shown
- Devices connecting → show spinner
- Unknown device type → generic device icon

**Technical Notes:**
- Component: `<BluetoothIndicator>` and `<BluetoothExpandedView>`
- Toggle: sends IPC command `SetBluetoothPowered { powered: bool }` → daemon calls BlueZ `Powered` property set
- Device type icons: headset, keyboard, mouse, phone, generic (based on `Class` or `Icon` property)

**Priority:** P2

---

## Feature 5.4: Volume Control

**Description:** Monitor and control system volume via PipeWire/WirePlumber D-Bus or ALSA. Display current volume level in pill and provide a slider in expanded view.

**Acceptance Criteria:** Volume icon shows in pill (muted/unmuted + level). Expanded view shows volume slider + mute toggle. Volume changes in real-time. Volume control works for default sink.

**Dependencies:** 1.2

**Notes:** Prefer PipeWire/WirePlumber over PulseAudio. Fall back to ALSA if needed.

### User Stories

#### US-5.4.1 — Volume Monitoring via PipeWire
**As a** system
**I want** to monitor system volume changes via PipeWire/WirePlumber
**so that** the Dynamic Island reflects current volume.

**Acceptance Criteria:**
- Connect to WirePlumber session manager via D-Bus `org.freedesktop.impl.portal.Settings` or PipeWire's D-Bus API
- Monitor default audio sink for: volume (0.0-1.0), mute state, device name
- Capture volume changes on `PropertiesChanged` signal
- Emit to UI via IPC within 200ms

**Edge Cases:**
- No audio hardware → volume indicator hidden
- Multiple sinks → monitor default sink only
- Volume exceeds 100% (some apps allow 150%) → clamp display to 100%

**Technical Notes:**
- Alternative: use `wpctl` (WirePlumber CLI) in a subprocess with `--watch` (less ideal, polling)
- Better: monitor `org.freedesktop.DBus.Properties` on PipeWire objects
- Or use `pulseaudio`'s D-Bus API `org.PulseAudio1` (deprecated but functional)
- Struct:
  ```rust
  struct VolumeState {
      volume: f64,       // 0.0 – 1.0
      muted: bool,
      sink_name: String,
  }
  ```

**Priority:** P1

#### US-5.4.2 — Volume Pill & Expanded Controls
**As an** end user
**I want** to see and control volume from the Dynamic Island
**so that** I can adjust audio without opening system settings.

**Acceptance Criteria:**
- Pill shows: [volume icon] + [percentage or level indicator]
  - Muted: speaker with X
  - Low (<33%): speaker with 1 wave
  - Medium (33-66%): speaker with 2 waves
  - High (>66%): speaker with 3 waves
- Expanded view shows:
  - Volume slider (0-100%, draggable)
  - Mute/unmute toggle button
  - Active output device name
- Slider responds in real-time (no delay)
- Dragging slider changes volume via D-Bus calls

**Edge Cases:**
- Mute toggle → slider goes to 0, icon changes
- Volume at 0 → always show muted icon regardless of mute state
- Keyboard volume keys → volume indicator updates (should already work via D-Bus monitoring)

**Technical Notes:**
- Component: `<VolumeIndicator>` and `<VolumeExpandedView>`
- Slider: custom `<input type="range">` styled to match dark theme, or custom Motion slider
- Volume change command: `SetVolume { volume: f64 }` → daemon calls `org.freedesktop.impl.portal.Settings.Set` or `wpctl set-volume`
- Mute command: `SetMute { muted: bool }`

**Priority:** P1

---

## Feature 5.5: Brightness Control

**Description:** Monitor and control screen brightness via D-Bus (`org.gnome.SettingsDaemon.Power.Screen` or `org.freedesktop.UPower`). Display brightness level and provide a slider in expanded view.

**Acceptance Criteria:** Brightness icon shows in pill. Expanded view shows brightness slider. Slider adjusts brightness in real-time. Works on laptops with backlight. Desktop without backlight hides indicator.

**Dependencies:** 1.2

**Notes:** Brightness control is laptop-specific. Desktop users will not see the indicator.

### User Stories

#### US-5.5.1 — Brightness Monitoring
**As a** system
**I want** to monitor screen brightness changes
**so that** the Dynamic Island reflects current brightness.

**Acceptance Criteria:**
- Connect to GNOME Settings Daemon power D-Bus: `org.gnome.SettingsDaemon.Power`
- Subscribe to `PropertiesChanged` on `/org/gnome/SettingsDaemon/Power`
- Capture: `Brightness` (0-100), `BrightnessSteps`
- Fallback: `org.freedesktop.UPower.DisplayDevice` brightness property
- Emit to UI via IPC within 200ms

**Edge Cases:**
- No backlight device (desktop monitor) → brightness indicator hidden
- External monitor with DDC/CI → not supported (read-only at best)

**Technical Notes:**
- Backlight detection: check `/sys/class/backlight/*` existence
- D-Bus path: `org.gnome.SettingsDaemon.Power` may not be running → check by name ownership
- Fallback: read/write `/sys/class/backlight/*/brightness` directly (requires permissions)
- Struct:
  ```rust
  struct BrightnessState {
      brightness: u8,     // 0-100
      has_backlight: bool,
  }
  ```

**Priority:** P1

#### US-5.5.2 — Brightness Pill & Slider
**As an** end user
**I want** to see and control screen brightness from the Dynamic Island
**so that** I can adjust it quickly.

**Acceptance Criteria:**
- Pill shows: [sun icon] + [percentage or no text, just icon]
- Icon brightness corresponds to level (opacity change)
- Expanded view shows:
  - Brightness slider (0-100%, draggable)
  - Auto-brightness toggle (if supported)
- Slider change → D-Bus call to adjust brightness
- Slider updates in real-time as brightness changes

**Edge Cases:**
- Brightness at 0% (screen off on some systems) → handle gracefully
- Slider animation should not lag behind actual brightness change

**Technical Notes:**
- Component: `<BrightnessIndicator>` and `<BrightnessExpandedView>`
- Brightness change command: `SetBrightness { brightness: u8 }` → daemon writes to `/sys/class/backlight/*/brightness`
- Auto-brightness: check if `org.gnome.SettingsDaemon.Power.Screen` has `AutoBrightness` property

**Priority:** P1

---

## Feature 5.6: Compact Status Icons in Pill

**Description:** Render all active system status indicators (battery, WiFi, Bluetooth, volume, brightness) as a compact row of icons in the Dynamic Island pill when there are no notifications or media active. Icons auto-hide when other content is showing.

**Acceptance Criteria:** Status icons appear in a row in the pill when no other activity is active. Icons are compact (16×16px each). Order is configurable. Icons auto-hide when media or notification content is active. Expanded view shows detailed controls.

**Dependencies:** 5.1, 5.2, 5.3, 5.4, 5.5, 2.1

**Notes:** This is the "dormant with status" state — the pill shows a row of system icons when nothing else is happening. Matches Apple's habit of showing status info in the island when idle.

### User Stories

#### US-5.6.1 — Status Icon Row
**As an** end user
**I want** to see a compact row of system status icons in the pill when idle
**so that** I can check battery, WiFi, and volume at a glance.

**Acceptance Criteria:**
- When no media or notification is active, pill shows a row of status icons
- Icons are 16×16px, evenly spaced
- Default order: Volume → WiFi → Bluetooth → Battery → Brightness
- Order is configurable via settings
- Only relevant icons shown (e.g., no battery on desktop)
- Row is centered in the pill

**Edge Cases:**
- All icons hidden (no features active) → show minimal pill (no status)
- Very many icons → some may be hidden with "+N" overflow indicator
- Icons should not animate (static display)

**Technical Notes:**
- Component: `<StatusIconRow>` renders a flex row of status icons
- Each icon component accepts `size` prop (16px for row)
- Visibility logic:
  ```typescript
  const visibleIcons = settings.statusOrder.filter(
    icon => icon.isAvailable && icon.isEnabled
  )
  ```
- Overflow: `max 5 icons visible + "+N"` or scale down spacing

**Priority:** P1

#### US-5.6.2 — Content-Aware Auto-Hide
**As a** system
**I want** status icons to automatically hide when media or notification content is active
**so that** the pill is not cluttered with both status and content.

**Acceptance Criteria:**
- When activity queue has media or notification item, status icons are replaced by that content
- When queue empties (all activities dismissed/expired), status icons reappear
- Transition: status icons fade out (100ms) → content fades in (150ms) and vice versa
- User can still access status details by expanding the island during content display (status is separate tab or accessible via swipe)

**Edge Cases:**
- Both media and notification active → content takes precedence
- User explicitly dismisses content → status icons return immediately

**Technical Notes:**
- `ActivityQueue` in React state determines what to display
- Priority: content (media/notification) > status icons
- Logic in `<Pill>` component:
  ```typescript
  if (currentActivity) render <ActivityContent activity={currentActivity} />
  else render <StatusIconRow />
  ```

**Priority:** P1

#### US-5.6.3 — Single-Tap Status Access
**As an** end user
**I want** to tap on a status icon to expand controls for that system feature
**so that** I can quickly adjust volume, toggle WiFi, or check battery details.

**Acceptance Criteria:**
- Tap on battery icon → expanded battery view
- Tap on WiFi icon → expanded WiFi view
- Tap on volume icon → expanded volume view with slider
- Tap on brightness icon → expanded brightness slider
- Expanded view replaces the status icon row (same pill, content changes)
- Tapping elsewhere or swiping returns to status icon row

**Edge Cases:**
- Status icon row not visible (content showing) → tapping the pill expands content, not status
- Rapid tapping between different status icons → smooth transitions

**Technical Notes:**
- Each status icon is wrapped in a clickable area
- Click → sets `selectedStatusFeature` state → pill renders that feature's expanded view
- Expanded status view uses the same `<ExpandedView>` container but with status content
- Back button (←) or swipe down returns to status row

**Priority:** P2
