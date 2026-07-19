# System Design — dyland

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        dyd-daemon (Rust)                         │
│                                                                  │
│  ┌─────────────────┐  ┌────────────────┐  ┌──────────────────┐  │
│  │   Event Bus      │  │  IPC Server    │  │  Plugin Runtime   │  │
│  │  (tokio broadcast)│  │  (Unix Socket) │  │  (wasmtime)      │  │
│  └────────┬────────┘  └───────┬────────┘  └────────┬─────────┘  │
│           │                   │                      │           │
│  ┌────────▼──────────────────────────────────────────▼─────────┐ │
│  │                      Event Sources                           │ │
│  │  ┌────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐ ┌──────┐  │ │
│  │  │ MPRIS  │ │ Notific. │ │ UPower   │ │ Netwkr │ │ BlueZ│  │ │
│  │  │ Listnr │ │ Monitor  │ │ Battery  │ │ Mgmt   │ │ BT   │  │ │
│  │  └────────┘ └──────────┘ └──────────┘ └────────┘ └──────┘  │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │                      Config Store                            │ │
│  │              $XDG_CONFIG_HOME/dyd/config.json                │ │
│  └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────┬───────────────────────────────────────┘
                           │ IPC (Unix Domain Socket)
                           │ Newline-delimited JSON
                           │ @dyd-daemon (abstract namespace)
┌──────────────────────────▼───────────────────────────────────────┐
│                       dyd-ui (Tauri + React)                     │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │                     IPC Client                               │ │
│  │              Persistent socket connection                    │ │
│  │              Reconnect with exponential backoff               │ │
│  └────────────────────────┬─────────────────────────────────────┘ │
│                           │                                       │
│  ┌────────────────────────▼─────────────────────────────────────┐ │
│  │                    State Store (Zustand)                     │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────┐ ┌────────┐ ┌────────┐  │ │
│  │  │Activity  │ │  Media   │ │ Notif│ │ System │ │Connection│  │ │
│  │  │ Queue    │ │  State   │ │ State│ │ Status │ │  State  │  │ │
│  │  └──────────┘ └──────────┘ └──────┘ └────────┘ └────────┘  │ │
│  └────────────────────────┬─────────────────────────────────────┘ │
│                           │                                       │
│  ┌────────────────────────▼─────────────────────────────────────┐ │
│  │                      UI Components                           │ │
│  │  ┌──────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐   │ │
│  │  │ Pill │ │ Expanded │ │ Settings │ │ Context Menu    │   │ │
│  │  │      │ │  View    │ │  Panel   │ │ (Long Press)    │   │ │
│  │  └──────┘ └──────────┘ └──────────┘ └──────────────────┘   │ │
│  └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## Process Architecture

### Two-Process Design Rationale

The application is split into two processes communicating over a Unix domain socket:

| Aspect | Daemon (`dyd-daemon`) | UI (`dyd-ui`) |
|---|---|---|
| **Language** | Rust | TypeScript/React |
| **Runtime** | Tokio (async) | Tauri/WebView |
| **Role** | System integration, state management, business logic | Rendering, animation, user interaction |
| **Startup** | Start first, or on login via systemd | Start by user or daemon-spawned |
| **Crash tolerance** | UI can restart without losing D-Bus subscriptions | Daemon can restart; UI reconnects |
| **Resource profile** | ~5-15MB RAM, near-zero CPU idle | ~40-70MB RAM (WebView), GPU for animations |

### Startup Sequence

```
User login
  └→ systemd starts dyd-daemon
       ├─ Bind IPC socket (@dyd-daemon)
       ├─ Connect to D-Bus session bus
       ├─ Start event listeners (MPRIS, Notifications, UPower, ...)
       ├─ Load plugins
       └─ Await UI connection

User launches dyd-ui (or auto-started)
  └→ Tauri WebView initializes
       ├─ Connect to @dyd-daemon
       ├─ Request full state snapshot
       ├─ Render pill in dormant state
       └─ Enter event loop
```

### Shutdown Sequence

```
SIGTERM/SIGINT to daemon
  ├─ Stop accepting new IPC connections
  ├─ Notify UI: daemon_shutting_down
  ├─ Flush event queues
  ├─ Disconnect D-Bus listeners
  ├─ Persist config and queue
  ├─ Close IPC socket
  └─ Exit 0

UI detects disconnect
  ├─ Show "disconnected" state
  ├─ Attempt reconnection (exponential backoff)
  └─ If daemon restarts: reconnect, resubscribe
```

---

## Crate/Module Structure (Rust)

```
dyd-daemon/
├── Cargo.toml
├── src/
│   ├── main.rs                    # Entry point, tokio runtime
│   ├── config/
│   │   ├── mod.rs                 # Config struct, load/save
│   │   └── schema.rs              # Default config, schema validation
│   ├── ipc/
│   │   ├── mod.rs                 # IPC server, connection handler
│   │   ├── server.rs             # Unix socket listener
│   │   ├── protocol.rs           # Message types (Event, Command, Response)
│   │   └── connection.rs         # Per-connection handler
│   ├── eventbus/
│   │   ├── mod.rs                 # Event bus (tokio broadcast)
│   │   ├── event.rs              # Event enum
│   │   └── subscription.rs       # Subscription handling
│   ├── sources/
│   │   ├── mod.rs                 # Event source trait
│   │   ├── mpris/
│   │   │   ├── mod.rs            # MPRIS listener
│   │   │   ├── player.rs         # MediaPlayer proxy
│   │   │   └── compat.rs         # Player-specific quirks
│   │   ├── notifications/
│   │   │   ├── mod.rs            # Notification monitor
│   │   │   └── dbus.rs           # org.freedesktop.Notifications proxy
│   │   ├── battery/
│   │   │   ├── mod.rs            # UPower listener
│   │   │   └── upower.rs         # UPower D-Bus proxy
│   │   ├── wifi/
│   │   │   ├── mod.rs            # NetworkManager listener
│   │   │   └── networkmanager.rs # NM D-Bus proxy
│   │   ├── bluetooth/
│   │   │   ├── mod.rs            # BlueZ listener
│   │   │   └── bluez.rs          # BlueZ D-Bus proxy
│   │   ├── volume/
│   │   │   ├── mod.rs            # Volume listener
│   │   │   └── pipewire.rs       # PipeWire/WirePlumber proxy
│   │   └── brightness/
│   │       ├── mod.rs            # Brightness listener
│   │       └── backlight.rs      # sysfs/D-Bus brightness
│   ├── queue/
│   │   ├── mod.rs                 # Activity queue
│   │   ├── activity.rs           # Activity struct
│   │   └── priority.rs           # Priority ordering
│   ├── plugins/
│   │   ├── mod.rs                 # Plugin loader
│   │   ├── manifest.rs           # Manifest parsing & validation
│   │   ├── runtime.rs            # WASM runtime (wasmtime)
│   │   └── host_functions.rs     # WASM host function bindings
│   └── lifecycle/
│       ├── mod.rs                 # Daemon lifecycle (start/shutdown)
│       └── health.rs             # Health check, metrics
```

---

## Component Tree (React)

```
<App>
  ├── <ThemeProvider>                    # Dark/Light/System theme context
  │   └── <ConnectionProvider>          # IPC connection state
  │       └── <StateProvider>           # Zustand stores
  │           └── <DynamicIsland>
  │               ├── <Pill>            # The always-visible pill
  │               │   ├── <Gestures>    # Click, swipe, long-press handling
  │               │   ├── <StatusIconRow> # System status icons (when idle)
  │               │   │   ├── <BatteryIcon>
  │               │   │   ├── <WifiIcon>
  │               │   │   ├── <BluetoothIcon>
  │               │   │   ├── <VolumeIcon>
  │               │   │   └── <BrightnessIcon>
  │               │   ├── <MediaPillContent>   # Media info (when media active)
  │               │   └── <NotificationPillContent> # Notification snippet
  │               │
  │               ├── <ExpandedView>    # The expanded island overlay
  │               │   ├── <MediaExpandedView>   # Full media controls
  │               │   ├── <NotificationExpandedView> # Full notification
  │               │   ├── <StatusExpandedView>  # System status detail
  │               │   │   ├── <BatteryDetail>
  │               │   │   ├── <VolumeSlider>
  │               │   │   └── <BrightnessSlider>
  │               │   └── <SettingsPanel>  # Settings view
  │               │
  │               ├── <ContextMenu>     # Long-press popup menu
  │               │
  │               └── <BackgroundOverlay> # Transparent click-catcher
  │
  └── <StyleProvider>                   # CSS variables, animations
```

---

## Data Flow: Key Scenarios

### Scenario 1: Media Playback Starts (Spotify)

```
Spotify app
  └→ D-Bus: PropertiesChanged on org.mpris.MediaPlayer2.spotify.Player
       └→ dyd-daemon: mpris::listener
            ├─ Parse Metadata (title, artist, album, artUrl)
            ├─ Update MediaState in daemon
            ├─ Push Activity { source: "media", content: metadata } to Queue
            └─ IPC Event: Event { type: "MediaChanged", payload: { ... } }
                 └→ dyd-ui: IPC client
                      ├─ Update Zustand store (media state)
                      ├─ Activity queue: new activity enqueued
                      ├─ Pill: transition dormant → active
                      │    ├─ Animate pill width: 126px → 180px
                      │    └─ Animate content: slide up → show [♪] Song Ti…
                      └─ If user clicks:
                           ├─ Pill: active → expanded
                           ├─ Animate pill: grow to 380px × 200px
                           ├─ Show MediaExpandedView
                           └─ Fetch album art via IPC → render
```

### Scenario 2: Notification Arrives

```
Messaging app
  └→ D-Bus: Notify on org.freedesktop.Notifications
       └→ dyd-daemon: notifications::monitor
            ├─ Parse app_name, summary, body, actions, hints
            ├─ Check DND mode (if on, queue silently or block)
            ├─ Check per-app settings
            ├─ Push Activity { source: "notification", ... } to Queue
            └─ IPC Event: Event { type: "NotificationReceived", payload: { ... } }
                 └→ dyd-ui:
                      ├─ Queue notification
                      ├─ If queue was empty: pill dormant → active
                      └─ Show notification content in pill
```

### Scenario 3: System Status Tick (Battery)

```
Laptop unplugged
  └→ D-Bus: PropertiesChanged on org.freedesktop.UPower.DisplayDevice
       └→ dyd-daemon: battery::upower
            └─ IPC Event: Event { type: "BatteryChanged", payload: { percentage: 85, state: "discharging" } }
                 └→ dyd-ui:
                      ├─ Update BatteryState in Zustand
                      └─ Pill: if idle, StatusIconRow re-renders with new battery %
```

### Scenario 4: User Interacts (Swipe to Dismiss)

```
User swipes down on active pill
  └→ Gesture recognizer: distance > 30px || velocity > 200px/s
       └→ IPC Command: Command { type: "DismissActivity", payload: { activity_id: "..." } }
            └→ dyd-daemon:
                 ├─ Remove activity from queue
                 ├─ If queue empty: emit QueueEmptied event
                 └─ IPC Response: { status: "ok" }
                      └→ dyd-ui:
                           ├─ Remove activity from store
                           ├─ If queue has next: animate transition
                           └─ If queue empty: pill → dormant
```

---

## IPC Protocol Specification

### Transport

- **Type:** Unix Domain Socket, abstract namespace
- **Address:** `@dyd-daemon` (Linux abstract socket, no filesystem entry)
- **Framing:** Newline-delimited JSON (`\n` as delimiter)
- **Max message size:** 1MB
- **Encoding:** UTF-8

### Message Envelope

Every message follows the same structure:

```json
{
  "type": "MessageType",
  "id": "uuid-v4",
  "timestamp": 1712345678.123,
  "payload": { }
}
```

- `type`: String, identifies the message kind
- `id`: UUID v4, used for correlating commands to responses
- `timestamp`: Unix timestamp with millisecond precision
- `payload`: Structured data specific to the message type

### Direction: Daemon → UI (Events)

Events are pushed from daemon to UI. They are not correlated to a previous command.

| Event Type | Payload | Description |
|---|---|---|
| `MediaChanged` | `{ player, title, artist, album, art_url, length, position, playback_status }` | Active media track changed |
| `MediaPosition` | `{ position_us }` | Periodic position update (every 1s) |
| `NotificationReceived` | `{ id, app_name, app_icon, summary, body, actions, urgency, timestamp }` | New notification |
| `NotificationClosed` | `{ id, reason }` | Notification dismissed/expired |
| `BatteryChanged` | `{ percentage, state, time_to_empty, time_to_full, is_present }` | Battery status update |
| `WifiChanged` | `{ ssid, strength, state, ip_address }` | WiFi status update |
| `BluetoothChanged` | `{ powered, connected_devices[] }` | Bluetooth status update |
| `VolumeChanged` | `{ volume, muted, sink_name }` | Volume level change |
| `BrightnessChanged` | `{ brightness, has_backlight }` | Brightness change |
| `ActivityQueued` | `{ activity_id, source, priority, content }` | New activity in queue |
| `ActivityDequeued` | `{ activity_id, reason }` | Activity removed from queue |
| `ActivityChanged` | `{ activity_id, source, content }` | Current activity changed |
| `QueueEmptied` | `{}` | Queue is empty (return to dormant) |
| `DndChanged` | `{ enabled }` | DND mode toggled |
| `FullscreenChanged` | `{ is_fullscreen, monitor }` | Fullscreen app detected/exited |
| `ConnectionStatus` | `{ status: "connected" \| "degraded" \| "shutting_down" }` | Daemon health |
| `PluginEvent` | `{ plugin, event_type, data }` | Plugin-generated event |
| `UpdateAvailable` | `{ version, url, size }` | New version available |
| `ConfigChanged` | `{ changes }` | Config change notification |

### Direction: UI → Daemon (Commands)

Commands are sent from UI to daemon. Daemon responds with a Response (same `id`).

| Command | Payload | Response | Description |
|---|---|---|---|
| `Ping` | `{}` | `{ status, uptime, version }` | Health check |
| `GetState` | `{}` | Full state snapshot | Request all current state |
| `DismissActivity` | `{ activity_id }` | `{ status }` | Dismiss an activity |
| `CycleNext` | `{}` | `{ activity_id }` | Show next activity in queue |
| `CyclePrev` | `{}` | `{ activity_id }` | Show previous activity |
| `MediaPlayPause` | `{ player }` | `{ status }` | Toggle play/pause |
| `MediaNext` | `{ player }` | `{ status }` | Next track |
| `MediaPrevious` | `{ player }` | `{ status }` | Previous track |
| `MediaSeek` | `{ player, position_us }` | `{ status }` | Seek to position |
| `MediaSetVolume` | `{ player, volume }` | `{ status }` | Set media player volume |
| `NotificationAction` | `{ id, action_key, reply_text? }` | `{ status }` | Invoke notification action |
| `NotificationClose` | `{ id }` | `{ status }` | Close notification |
| `SetDnd` | `{ enabled }` | `{ status }` | Toggle DND |
| `SetVolume` | `{ volume }` | `{ status }` | Set system volume |
| `SetMute` | `{ muted }` | `{ status }` | Toggle mute |
| `SetBrightness` | `{ brightness }` | `{ status }` | Set brightness |
| `SetBluetoothPowered` | `{ powered }` | `{ status }` | Toggle Bluetooth |
| `GetConfig` | `{}` | `{ config }` | Get full config |
| `UpdateConfig` | `{ changes }` | `{ config }` | Update config (partial) |
| `GetPlugins` | `{}` | `{ plugins[] }` | List installed plugins |
| `SetPluginEnabled` | `{ name, enabled }` | `{ status }` | Enable/disable plugin |
| `UninstallPlugin` | `{ name }` | `{ status }` | Uninstall plugin |
| `InstallPlugin` | `{ path }` | `{ status }` | Install plugin from path |

### Response Format

```json
{
  "type": "Response",
  "id": "corresponding-command-id",
  "timestamp": 1712345678.456,
  "payload": {
    "status": "ok" | "error",
    "data": { },
    "error": { "code": "string", "message": "string" }  // only on error
  }
}
```

---

## State Machine

### Pill States

```
                  ┌──────────────┐
        ┌────────►│   DORMANT    │◄──────────┐
        │         │ 126×36px    │           │
        │         │ no content   │           │
        │         │ input pass   │           │
        │         └──────┬───────┘           │
        │                │                    │
        │     new activity│                   │
        │                ▼                    │
        │         ┌──────────────┐           │
        │         │    ACTIVE    │           │
        │         │ flex width   │────────────┘
        │  ┌─────►│ icon+1-line  │   dismiss/all done
        │  │      │ interactive  │
        │  │      └──────┬───────┘
        │  │             │
        │  │    click    │ swipe down
        │  │             │
        │  │      ┌──────▼───────┐
        │  │      │   EXPANDED   │
        │  │      │ 380×200+px   │
        │  │      │ full content │
        │  │      │ all controls │
        │  │      └──────┬───────┘
        │  │             │
        │  │   click     │ swipe down
        │  │   outside   │
        │  │      ┌──────▼───────┐
        │  └──────┤    ACTIVE    │
        └─────────┘             │
                  └──────────────┘

All states can transition to HIDDEN (fullscreen detected)
HIDDEN returns to previous state (or DORMANT if state lost)
```

### State Transition Effects

| Transition | Duration | Easing | Content Animation |
|---|---|---|---|
| DORMANT → ACTIVE | 300ms | spring (stiffness:400, damping:28) | Slide up + fade in, delay 50ms |
| ACTIVE → DORMANT | 200ms | spring (stiffness:350, damping:30) | Slide down + fade out |
| ACTIVE → EXPANDED | 400ms | spring (stiffness:300, damping:25) | Content scales + fades |
| EXPANDED → ACTIVE | 300ms | spring (stiffness:350, damping:25) | Content shrinks, blur fades |
| → HIDDEN (any) | 100ms | ease-out | Slide up off screen |
| HIDDEN → (prev) | 150ms | ease-out | Slide down from off screen |

---

## Plugin Architecture

### Plugin Lifecycle

```
┌──────────┐   validate    ┌──────────┐   init()    ┌──────────┐
│ DISCOVERED│─────────────►│  LOADED  │────────────►│INITIALIZED│
└──────────┘               └──────────┘             └─────┬─────┘
                                                           │ start()
                                                           ▼
┌──────────┐   stop()     ┌──────────┐              ┌──────────┐
│ STOPPED  │◄─────────────│ RUNNING  │◄─────────────│ RUNNING  │
└─────┬────┘              └──────────┘              └──────────┘
      │                                                  │
      │ unload()                                         │ panic/trap
      ▼                                                  ▼
┌──────────┐                                       ┌──────────┐
│ UNLOADED │                                       │  ERROR   │
└──────────┘                                       └─────┬────┘
                                                          │ reset()
                                                          ▼
                                                    ┌──────────┐
                                                    │  LOADED  │
                                                    └──────────┘
```

### WASM Host API

The daemon exposes the following host functions to WASM plugins:

```rust
// Logging
fn log_info(msg: &str);
fn log_warn(msg: &str);
fn log_error(msg: &str);

// Configuration
fn config_get(key: &str) -> Result<Option<String>>;
fn config_set(key: &str, value: &str) -> Result<()>;

// Activity emission
fn emit_activity(source: &str, content_json: &str) -> Result<()>;

// Event subscription
fn subscribe(event_type: &str, callback_id: u32) -> Result<()>;
fn unsubscribe(event_type: &str, callback_id: u32) -> Result<()>;

// HTTP client (requires permission)
fn http_request(url: &str, options_json: &str) -> Result<String>;

// D-Bus (requires permission)
fn dbus_call(service: &str, path: &str, interface: &str, method: &str, args_json: &str) -> Result<String>;

// Timer
fn create_timer(interval_ms: u64, callback_id: u32) -> Result<u32>;
fn cancel_timer(timer_id: u32) -> Result<()>;
```

### Sandbox Restrictions

| Resource | Default | With Permission |
|---|---|---|
| Filesystem | None | Unix socket paths listed in manifest |
| Network | None | HTTP to hosts listed in manifest |
| D-Bus | None | Specific services listed in manifest |
| Memory | 10MB max | - |
| CPU | 100k fuel units per tick | Configurable |
| Execution time per call | 5s timeout | - |
| Logging | Full | Full |
| Subprocesses | Not allowed | Not allowed |

---

## Security Model

### Threat Model

| Threat | Mitigation |
|---|---|
| Malicious plugin accesses filesystem | WASM sandbox, no fs access by default |
| Malicious plugin network exfiltration | HTTP allowed only to permitted hosts |
| Malicious plugin D-Bus abuse | D-Bus access only to permitted services |
| IPC eavesdropping | Unix socket, file permissions 0700 |
| Config tampering | Config owned by user, atomic writes |
| Daemon crash → UI confusion | UI detects disconnect, shows indicator |

### D-Bus Security

- Daemon connects to session bus only (not system bus)
- All D-Bus operations are read-only unless user explicitly acts (e.g., volume control requires user gesture)
- Plugin D-Bus calls are filtered by manifest permissions

### IPC Security

- Socket is in abstract namespace — no filesystem entry
- Only accessible by the same user (unix socket permissions)
- Message size limit: 1MB (prevent memory exhaustion)
- Rate limit: 1000 messages/second per connection (prevent DoS)

---

## Error Handling Strategy

### Error Classification

| Class | Example | Response |
|---|---|---|
| Transient | D-Bus service temporarily unavailable | Retry with exponential backoff, max 5 attempts |
| Permanent | No battery hardware, no WiFi interface | Feature gracefully disabled, hide indicator |
| User-actionable | Permission denied, config corrupt | User notification via island |
| Internal | Plugin crash, deserialization failure | Log error, disable failing component |

### Graceful Degradation Matrix

| Failure | Degraded Behavior |
|---|---|
| MPRIS listener fails | Media features hidden, other features continue |
| Notification monitor fails | Native notifications work as normal |
| Battery monitor fails | Battery icon hidden |
| All system status fails | Pill shows minimal (no icons) |
| Plugin crashes | Single plugin disabled, others continue |
| IPC connection lost | UI shows "Disconnected" state |
| WebView crashes | Daemon continues running, restart UI |
| Config file corrupt | Use defaults, log warning |

---

## Performance Targets

| Metric | Target | Measurement Method |
|---|---|---|
| Startup (daemon + UI first frame) | <150ms | tracing spans + Chrome DevTools |
| Idle CPU (daemon) | <0.5% | perf stat, htop over 5 min |
| Idle CPU (UI) | <0.5% | Chrome DevTools Performance |
| Idle RAM (daemon) | <15MB RSS | heaptrack |
| Idle RAM (UI) | <70MB RSS | Chrome DevTools Memory |
| Animation frame rate | 60fps (16ms per frame) | Chrome DevTools FPS meter |
| Event processing latency | <1ms per event | tracing + histogram |
| IPC latency (round trip) | <1ms localhost | Built-in latency probe |
| D-Bus → UI render latency | <5ms end-to-end | tracing spans |

---

## Configuration Schema

Config stored at `$XDG_CONFIG_HOME/dyd/config.json`:

```json
{
  "$schema": "https://dyland.app/schemas/config-v1.json",
  "version": 1,
  "general": {
    "start_on_login": true,
    "language": "en",
    "theme": "dark"
  },
  "appearance": {
    "theme": "dark",
    "reduce_motion": false,
    "high_contrast": false,
    "font_scale": 1.0
  },
  "notifications": {
    "suppress_native": true,
    "dnd": {
      "enabled": false,
      "schedule": [],
      "manual_override": null
    },
    "per_app": {}
  },
  "media": {
    "enabled": true,
    "show_album_art": true,
    "preferred_player": null
  },
  "system_status": {
    "battery": { "enabled": true },
    "wifi": { "enabled": true },
    "bluetooth": { "enabled": true },
    "volume": { "enabled": true },
    "brightness": { "enabled": true }
  },
  "status_bar": {
    "icon_order": ["volume", "wifi", "bluetooth", "battery", "brightness"]
  },
  "accessibility": {
    "reduce_motion": false,
    "high_contrast": false,
    "font_scale": 1.0
  },
  "plugins": {
    "enabled_plugins": {}
  }
}
```
