# Implementation Strategy — dyland

## Dependency Graph

```
E1: Foundation ──────────────────────────────────────────────────┐
  ├─ 1.1 Scaffolding & Tooling                                   │
  ├─ 1.2 Daemon Core (lifecycle, D-Bus) ─┐                      │
  ├─ 1.3 UI Shell (Tauri, transparent) ──┤                      │
  ├─ 1.4 IPC Bridge ─────────────────────┤                      │
  ├─ 1.5 Pill Placeholder ───────────────┤                      │
  └─ 1.6 Session Management ─────────────┘                      │
                                                                 │
E2: Pill Rendering ──────────────────────────────────────────────┤
  ├─ 2.1 State Machine          (depends: 1.4, 1.5)             │
  ├─ 2.2 Animations             (depends: 2.1)                  │
  ├─ 2.3 Gestures               (depends: 2.1)                  │
  ├─ 2.4 Activity Queue         (depends: 2.1, 1.4)             │
  ├─ 2.5 Hover & Focus          (depends: 2.1)                  │
  └─ 2.6 Fullscreen Detection   (depends: 1.2, 2.1)             │
                                                                 │
E3: Media ───────────────────────────────────────────────────────┤
  ├─ 3.1 MPRIS Listener          (depends: 1.2, 1.4)            │
  ├─ 3.2 Now-Playing UI         (depends: 2.1, 2.2, 3.1)        │
  ├─ 3.3 Playback Controls      (depends: 3.1)                  │
  ├─ 3.4 Album Art              (depends: 3.2)                  │
  └─ 3.5 Multi-Source           (depends: 3.1, 3.2)             │
                                                                 │
E4: Notifications ───────────────────────────────────────────────┤
  ├─ 4.1 D-Bus Monitor          (depends: 1.2, 1.4)             │
  ├─ 4.2 Notification UI        (depends: 2.1, 2.2, 4.1)        │
  ├─ 4.3 Actions & Reply        (depends: 4.2)                  │
  ├─ 4.4 Grouping               (depends: 4.2)                  │
  └─ 4.5 DND                    (depends: 4.1)                  │
                                                                 │
E5: System Status ───────────────────────────────────────────────┤
  ├─ 5.1 Battery                (depends: 1.2, 1.4)             │
  ├─ 5.2 WiFi                   (depends: 1.2, 1.4)             │
  ├─ 5.3 Bluetooth              (depends: 1.2, 1.4)             │
  ├─ 5.4 Volume                 (depends: 1.2, 1.4)             │
  ├─ 5.5 Brightness             (depends: 1.2, 1.4)             │
  └─ 5.6 Status Row             (depends: 2.1, 5.1-5.5)         │
                                                                 │
E6: Plugin System ───────────────────────────────────────────────┤
  ├─ 6.1 Manifest               (depends: none, design only)    │
  ├─ 6.2 Lifecycle              (depends: 1.2, 6.1)             │
  ├─ 6.3 WASM Runtime           (depends: 6.2)                  │
  ├─ 6.4 Plugin SDK             (depends: 6.3)                  │
  ├─ 6.5 Plugin Mgmt UI         (depends: 6.2, 7.1)             │
  └─ 6.6 Example Plugins        (depends: 6.4)                  │
                                                                 │
E7: Settings ────────────────────────────────────────────────────┤
  ├─ 7.1 Settings UI            (depends: 2.3, 1.4)             │
  ├─ 7.2 Theme System           (depends: 7.1)                  │
  ├─ 7.3 Feature Toggles        (depends: 7.1)                  │
  ├─ 7.4 Keyboard Shortcuts     (depends: 7.1, 2.1)             │
  └─ 7.5 Accessibility          (depends: 7.1)                  │
                                                                 │
E8: Polish ──────────────────────────────────────────────────────┤
  ├─ 8.1 Animation Polish       (depends: 2.2, 3.2, 4.2)        │
  ├─ 8.2 Startup Perf           (depends: 1.2, 1.3, 1.4)        │
  ├─ 8.3 Runtime Perf           (depends: all sources)           │
  ├─ 8.4 Edge Cases             (depends: all features)          │
  └─ 8.5 Visual Polish          (depends: all features)          │
                                                                 │
E9: Distribution ────────────────────────────────────────────────┤
  ├─ 9.1 .deb Package           (depends: all features)          │
  ├─ 9.2 AppImage               (depends: 9.1)                  │
  ├─ 9.3 CI/CD                  (depends: 1.1)                   │
  ├─ 9.4 Auto-Update            (depends: 9.1, 9.3)             │
  └─ 9.5 Documentation          (depends: all features)          │
```

---

## Implementation Phases

### Phase 1: Skeleton (Weeks 1-2)
**Goal:** Running daemon + UI with empty pill on screen.

| Order | Feature | Effort | Risk | Key Deliverable |
|---|---|---|---|---|
| 1 | 1.1 Project Scaffolding | 1 day | Low | Monorepo, Cargo + npm workspaces |
| 2 | 1.2 Daemon Core | 2 days | Medium | Running daemon with D-Bus connection |
| 3 | 1.3 UI Shell (Tauri) | 3 days | **High** | Transparent overlay window (Wayland) |
| 4 | 1.4 IPC Bridge | 2 days | Medium | Bidirectional Unix socket communication |
| 5 | 1.5 Pill Placeholder | 1 day | Low | Static black pill visible on screen |
| 6 | 1.6 Session Management | 1 day | Low | systemd service, single instance |

**Gate: Demoable — black pill on screen, daemon running, IPC working**

### Phase 2: Heart (Weeks 3-5)
**Goal:** Interactive Dynamic Island with media support.

| Order | Feature | Effort | Risk | Key Deliverable |
|---|---|---|---|---|
| 7 | 2.1 State Machine | 2 days | Low | Dormant/Active/Expanded states working |
| 8 | 2.2 Animations | 3 days | Medium | Spring-based pill transitions at 60fps |
| 9 | 2.3 Gestures | 2 days | Medium | Click, swipe, long-press working |
| 10 | 2.4 Activity Queue | 2 days | Medium | Multi-activity priority queuing |
| 11 | 3.1 MPRIS Listener | 2 days | Medium | Detects Spotify, Firefox, etc. |
| 12 | 3.2 Now-Playing UI | 2 days | Low | Media info in pill + expanded view |
| 13 | 3.3 Playback Controls | 1 day | Low | Play/Pause/Next/Previous working |
| 14 | 3.4 Album Art | 1 day | Low | Art fetching and rendering |
| 15 | 3.5 Multi-Source Media | 1 day | Low | Player switching |

**Gate: Demoable — media plays in island, controls work, animations smooth**

### Phase 3: Connected (Weeks 6-8)
**Goal:** Notifications + system status integrated.

| Order | Feature | Effort | Risk | Key Deliverable |
|---|---|---|---|---|
| 16 | 4.1 Notification Monitor | 3 days | **High** | Intercepting GNOME notifications |
| 17 | 4.2 Notification UI | 2 days | Low | Notifications in pill + expanded view |
| 18 | 4.3 Actions & Reply | 2 days | Medium | Action buttons, inline reply |
| 19 | 4.4 Grouping | 1 day | Low | App-based notification groups |
| 20 | 4.5 DND Mode | 1 day | Low | DND toggle + scheduling |
| 21 | 5.1 Battery | 1 day | Low | UPower monitoring |
| 22 | 5.2 WiFi | 1 day | Low | NetworkManager monitoring |
| 23 | 5.3 Bluetooth | 1 day | Low | BlueZ monitoring |
| 24 | 5.4 Volume | 1 day | Low | PipeWire volume control |
| 25 | 5.5 Brightness | 1 day | Low | Backlight control |
| 26 | 5.6 Status Row | 1 day | Low | Compact status icons in pill |
| 27 | 2.5 Hover & Focus | 1 day | Low | Hover glow, keyboard navigation |
| 28 | 2.6 Fullscreen Detection | 2 days | **High** | Hide pill during fullscreen apps |

**Gate: Demoable — full Dynamic Island experience with media, notifications, system status**

### Phase 4: Configurable (Weeks 9-10)
**Goal:** Settings, theming, plugin system, accessibility.

| Order | Feature | Effort | Risk | Key Deliverable |
|---|---|---|---|---|
| 29 | 7.1 Settings UI | 3 days | Medium | Settings panel in expanded view |
| 30 | 7.2 Theme System | 2 days | Low | Dark/light/system themes |
| 31 | 7.3 Feature Toggles | 1 day | Low | Per-feature enable/disable |
| 32 | 7.4 Keyboard Shortcuts | 2 days | Low | Global + per-window shortcuts |
| 33 | 7.5 Accessibility | 2 days | Medium | ARIA, high contrast, font scale |
| 34 | 6.1 Plugin Manifest | 1 day | Low | Schema design + validation |
| 35 | 6.2 Plugin Lifecycle | 3 days | **High** | Load/init/start/stop plugins |
| 36 | 6.3 WASM Runtime | 4 days | **High** | wasmtime integration + sandbox |
| 37 | 6.4 Plugin SDK | 2 days | Medium | Rust SDK crate + host functions |
| 38 | 6.5 Plugin Mgmt UI | 2 days | Low | Plugin list, enable/disable in settings |
| 39 | 6.6 Example Plugins | 2 days | Medium | Docker, Git, Terminal plugins |

**Gate: Demoable — settings work, plugins installable, themes change appearance**

### Phase 5: Polished (Weeks 11-12)
**Goal:** Performance optimization, edge case handling, visual refinement.

| Order | Feature | Effort | Risk | Key Deliverable |
|---|---|---|---|---|
| 40 | 8.1 Animation Polish | 2 days | Medium | Spring tuning, staggered transitions |
| 41 | 8.2 Startup Performance | 2 days | Medium | <150ms startup target |
| 42 | 8.3 Runtime Performance | 2 days | Medium | <100MB RAM, near-zero idle CPU |
| 43 | 8.4 Edge Case Handling | 3 days | Medium | Graceful degradation matrix covered |
| 44 | 8.5 Visual Polish | 2 days | Low | Icons, typography, spacing audit |

**Gate: Demoable — meets all performance targets**

### Phase 6: Shipped (Week 13)
**Goal:** Package, document, release.

| Order | Feature | Effort | Risk | Key Deliverable |
|---|---|---|---|---|
| 45 | 9.1 .deb Package | 2 days | Medium | Installable on Ubuntu 24.04 |
| 46 | 9.2 AppImage | 1 day | Low | Distribution-agnostic binary |
| 47 | 9.3 CI/CD Pipeline | 2 days | Low | Automated builds + releases |
| 48 | 9.4 Auto-Update | 2 days | Medium | Update check + install |
| 49 | 9.5 Documentation | 3 days | Low | User guide, developer docs |

**Gate: v1.0.0 release on GitHub**

---

## API Design Specification

### Event Types (Daemon → UI)

```typescript
// === SYSTEM EVENTS ===

interface DaemonConnected {
  type: 'DaemonConnected'
  payload: {
    version: string
    uptime_secs: number
    features_available: string[]
  }
}

interface DaemonShuttingDown {
  type: 'DaemonShuttingDown'
  payload: {
    reason: 'signal' | 'crash' | 'upgrade'
  }
}

// === MEDIA EVENTS ===

interface MediaChanged {
  type: 'MediaChanged'
  payload: {
    player: string              // bus name
    player_name: string         // human name
    title: string
    artist: string[]
    album: string
    art_url: string | null
    length_us: number
    position_us: number
    playback_status: 'playing' | 'paused' | 'stopped'
    can_play: boolean
    can_pause: boolean
    can_go_next: boolean
    can_go_previous: boolean
    can_seek: boolean
  }
}

interface MediaPosition {
  type: 'MediaPosition'
  payload: {
    player: string
    position_us: number
  }
}

interface MediaPlayersChanged {
  type: 'MediaPlayersChanged'
  payload: {
    players: Array<{
      name: string
      identity: string
      playback_status: string
    }>
  }
}

// === NOTIFICATION EVENTS ===

interface NotificationReceived {
  type: 'NotificationReceived'
  payload: {
    id: number
    app_name: string
    app_icon: string
    summary: string
    body: string
    actions: string[]              // [key, label, key, label, ...]
    urgency: 'low' | 'normal' | 'critical'
    category: string
    timestamp: number              // unix ms
    has_inline_reply: boolean
    inline_reply_placeholder: string
  }
}

interface NotificationClosed {
  type: 'NotificationClosed'
  payload: {
    id: number
    reason: 'expired' | 'dismissed' | 'app_cancelled'
  }
}

// === SYSTEM STATUS EVENTS ===

interface BatteryChanged {
  type: 'BatteryChanged'
  payload: {
    percentage: number             // 0-100
    state: 'unknown' | 'charging' | 'discharging' | 'empty' | 'full'
    time_to_empty_secs: number | null
    time_to_full_secs: number | null
    is_present: boolean
  }
}

interface WifiChanged {
  type: 'WifiChanged'
  payload: {
    ssid: string | null
    strength: number               // 0-100
    state: 'connected' | 'connecting' | 'disconnected' | 'disabled'
    ip_address: string | null
  }
}

interface BluetoothChanged {
  type: 'BluetoothChanged'
  payload: {
    powered: boolean
    connected_devices: Array<{
      name: string
      address: string
      icon: string
    }>
  }
}

interface VolumeChanged {
  type: 'VolumeChanged'
  payload: {
    volume: number                 // 0.0 - 1.0
    muted: boolean
    sink_name: string
    sink_description: string
  }
}

interface BrightnessChanged {
  type: 'BrightnessChanged'
  payload: {
    brightness: number             // 0-100
    has_backlight: boolean
  }
}

// === ACTIVITY QUEUE EVENTS ===

interface ActivityQueued {
  type: 'ActivityQueued'
  payload: {
    activity_id: string
    source: 'media' | 'notification' | 'system_status' | 'plugin'
    priority: number               // 0-100, higher = more important
    content: Record<string, unknown>
    timestamp: number
  }
}

interface ActivityDequeued {
  type: 'ActivityDequeued'
  payload: {
    activity_id: string
    reason: 'dismissed' | 'expired' | 'completed'
  }
}

interface CurrentActivityChanged {
  type: 'CurrentActivityChanged'
  payload: {
    activity_id: string | null
    activity: ActivityQueued['payload'] | null
    queue_length: number
    queue_position: number
  }
}

// === STATE EVENTS ===

interface DndChanged {
  type: 'DndChanged'
  payload: {
    enabled: boolean
    until: number | null           // unix ms if scheduled end
  }
}

interface FullscreenChanged {
  type: 'FullscreenChanged'
  payload: {
    is_fullscreen: boolean
    monitor: string | null
  }
}

interface ConfigChanged {
  type: 'ConfigChanged'
  payload: {
    changes: Record<string, unknown>
  }
}

interface PluginEvent {
  type: 'PluginEvent'
  payload: {
    plugin: string
    event_type: string
    data: Record<string, unknown>
  }
}

interface UpdateAvailable {
  type: 'UpdateAvailable'
  payload: {
    version: string
    url: string
    size_bytes: number
    release_notes: string
  }
}

// === HEALTH EVENTS ===

interface HealthStatus {
  type: 'HealthStatus'
  payload: {
    daemon_uptime_secs: number
    connected_clients: number
    active_listeners: string[]
    plugins_loaded: number
    plugins_error: number
    memory_usage_kb: number
  }
}

// === Union type for all events ===

type DaemonEvent =
  | DaemonConnected
  | DaemonShuttingDown
  | MediaChanged
  | MediaPosition
  | MediaPlayersChanged
  | NotificationReceived
  | NotificationClosed
  | BatteryChanged
  | WifiChanged
  | BluetoothChanged
  | VolumeChanged
  | BrightnessChanged
  | ActivityQueued
  | ActivityDequeued
  | CurrentActivityChanged
  | DndChanged
  | FullscreenChanged
  | ConfigChanged
  | PluginEvent
  | UpdateAvailable
  | HealthStatus
```

### Command Types (UI → Daemon)

```typescript
// === COMMANDS ===

interface Ping {
  type: 'Ping'
  payload: Record<string, never>
}

interface GetState {
  type: 'GetState'
  payload: Record<string, never>
}

interface DismissActivity {
  type: 'DismissActivity'
  payload: { activity_id: string }
}

interface CycleNext {
  type: 'CycleNext'
  payload: Record<string, never>
}

interface CyclePrev {
  type: 'CyclePrev'
  payload: Record<string, never>
}

// Media controls
interface MediaPlayPause {
  type: 'MediaPlayPause'
  payload: { player: string }
}
interface MediaNext {
  type: 'MediaNext'
  payload: { player: string }
}
interface MediaPrevious {
  type: 'MediaPrevious'
  payload: { player: string }
}
interface MediaSeek {
  type: 'MediaSeek'
  payload: { player: string; position_us: number }
}

// Notification actions
interface NotificationAction {
  type: 'NotificationAction'
  payload: {
    notification_id: number
    action_key: string
    reply_text?: string
  }
}
interface NotificationClose {
  type: 'NotificationClose'
  payload: { notification_id: number }
}

// System controls
interface SetDnd {
  type: 'SetDnd'
  payload: { enabled: boolean }
}
interface SetVolume {
  type: 'SetVolume'
  payload: { volume: number }
}
interface SetMute {
  type: 'SetMute'
  payload: { muted: boolean }
}
interface SetBrightness {
  type: 'SetBrightness'
  payload: { brightness: number }
}
interface SetBluetoothPowered {
  type: 'SetBluetoothPowered'
  payload: { powered: boolean }
}

// Config
interface GetConfig {
  type: 'GetConfig'
  payload: Record<string, never>
}
interface UpdateConfig {
  type: 'UpdateConfig'
  payload: { changes: Record<string, unknown> }
}

// Plugin management
interface GetPlugins {
  type: 'GetPlugins'
  payload: Record<string, never>
}
interface SetPluginEnabled {
  type: 'SetPluginEnabled'
  payload: { name: string; enabled: boolean }
}
interface InstallPlugin {
  type: 'InstallPlugin'
  payload: { path: string }
}
interface UninstallPlugin {
  type: 'UninstallPlugin'
  payload: { name: string }
}
interface GetPluginConfig {
  type: 'GetPluginConfig'
  payload: { name: string }
}
interface SetPluginConfig {
  type: 'SetPluginConfig'
  payload: { name: string; config: Record<string, unknown> }
}

// Union type for all commands
type DaemonCommand =
  | Ping
  | GetState
  | DismissActivity
  | CycleNext
  | CyclePrev
  | MediaPlayPause
  | MediaNext
  | MediaPrevious
  | MediaSeek
  | NotificationAction
  | NotificationClose
  | SetDnd
  | SetVolume
  | SetMute
  | SetBrightness
  | SetBluetoothPowered
  | GetConfig
  | UpdateConfig
  | GetPlugins
  | SetPluginEnabled
  | InstallPlugin
  | UninstallPlugin
  | GetPluginConfig
  | SetPluginConfig
```

### Response Types

```typescript
interface CommandResponse {
  type: 'Response'
  id: string                // matches command id
  payload: {
    status: 'ok' | 'error'
    data?: Record<string, unknown>
    error?: {
      code: string          // machine-readable: "DBUS_ERROR", "NOT_FOUND"
      message: string       // human-readable
    }
  }
}
```

### State Snapshot (response to GetState)

```typescript
interface StateSnapshot {
  daemon: {
    version: string
    uptime_secs: number
    features_available: string[]
  }
  media: {
    active_player: string | null
    players: Array<{
      name: string
      identity: string
      playback_status: string
      title: string
      artist: string[]
      album: string
      art_url: string | null
      length_us: number
      position_us: number
      capabilities: {
        can_play: boolean
        can_pause: boolean
        can_go_next: boolean
        can_go_previous: boolean
        can_seek: boolean
      }
    }>
  }
  notifications: {
    groups: Array<{
      app_name: string
      app_icon: string
      notifications: Array<{
        id: number
        summary: string
        body: string
        timestamp: number
        urgency: string
      }>
    }>
  }
  system: {
    battery: BatteryChanged['payload'] | null
    wifi: WifiChanged['payload'] | null
    bluetooth: BluetoothChanged['payload'] | null
    volume: VolumeChanged['payload'] | null
    brightness: BrightnessChanged['payload'] | null
  }
  queue: {
    current: CurrentActivityChanged['payload']
    items: Array<ActivityQueued['payload']>
  }
  config: Record<string, unknown>
  plugins: Array<{
    name: string
    version: string
    state: 'running' | 'error' | 'disabled'
    config: Record<string, unknown>
  }>
}
```

---

## Testing Strategy

### Levels

| Level | Tool | Scope | Target |
|---|---|---|---|
| Unit (Rust) | built-in `#[test]` | Individual functions, modules, D-Bus proxy parsing | >80% coverage on business logic |
| Unit (TypeScript) | Vitest | React components, hooks, store logic | >80% coverage on store + utils |
| Integration (Rust) | `#[tokio::test]` | IPC protocol, queue logic, event bus | Key flows: event → process → emit |
| Integration (Tauri) | `@tauri-apps/test` | Tauri commands, window management | Window positioning, focus behavior |
| E2E | Custom test harness | Daemon + UI together, simulated D-Bus | Full lifecycle, multi-scenario |

### Testing the IPC Protocol

```rust
// Integration test for IPC (Rust side)
#[tokio::test]
async fn test_ipc_round_trip() {
    let daemon = TestDaemon::start().await;
    let client = TestIpcClient::connect().await;

    client.send(Ping).await;
    let response = client.receive().await;
    assert_eq!(response.status, "ok");
    assert!(response.data.uptime_secs >= 0);
}
```

### Testing with Mock D-Bus

```rust
// Mock D-Bus server for testing event sources
#[tokio::test]
async fn test_mpris_listener() {
    let mut mock_bus = MockDBusServer::new().await;
    mock_bus.add_service("org.mpris.MediaPlayer2.spotify");

    let daemon = TestDaemon::with_bus(mock_bus.session()).await;
    let client = TestIpcClient::connect().await;

    // Simulate a track change
    mock_bus.emit_properties_changed("org.mpris.MediaPlayer2.spotify.Player", json!({
        "Metadata": { "xesam:title": "Test Song" }
    }));

    let event = client.wait_for_event("MediaChanged").await;
    assert_eq!(event.payload.title, "Test Song");
}
```

### Testing the UI (Component Tests)

```typescript
// Vitest + React Testing Library
describe('Pill', () => {
  it('renders dormant state with correct dimensions', () => {
    const { container } = render(<Pill state="dormant" />)
    const pill = container.firstChild
    expect(pill).toHaveStyle({ width: '126px', height: '36px' })
  })

  it('expands on state change with spring animation', async () => {
    const { rerender, container } = render(<Pill state="dormant" />)
    rerender(<Pill state="active" />)
    // Animation is async — check that motion props change
    await waitFor(() => {
      expect(container.firstChild).toHaveAttribute('data-animate-width')
    })
  })

  it('renders status icons when no activity', () => {
    render(
      <StateProvider overrides={{ currentActivity: null }}>
        <Pill state="dormant" />
      </StateProvider>
    )
    expect(screen.getByTestId('status-icon-row')).toBeInTheDocument()
  })
})
```

### Manual Testing Checklist

Create `docs/testing/checklist.md` with:

```
## Smoke Test (after every change)
- [ ] Daemon starts without error
- [ ] UI connects to daemon
- [ ] Pill visible at top-center
- [ ] No console errors

## Media
- [ ] Spotify track shows in pill
- [ ] Play/Pause works
- [ ] Next/Previous works
- [ ] Album art displays
- [ ] Progress bar updates
- [ ] Multiple players: switching works

## Notifications
- [ ] Notification appears in pill
- [ ] Expand shows full content
- [ ] Action buttons work
- [ ] Inline reply works (if app supports)
- [ ] DND suppresses non-critical
- [ ] Critical notifications bypass DND

## System Status
- [ ] Battery percentage updates
- [ ] WiFi SSID displays
- [ ] Volume slider changes volume
- [ ] Brightness slider changes brightness

## Gestures
- [ ] Click expands pill
- [ ] Swipe dismisses activity
- [ ] Long-press shows context menu
- [ ] Click outside collapses

## Performance
- [ ] Startup < 500ms (v1 target)
- [ ] Animations smooth (no jank)
- [ ] No memory growth over 1 hour
```

---

## Performance Budget

| Metric | Budget | Measurement |
|---|---|---|
| `cargo build` (debug) | <60s | `time cargo build` |
| `npm run build` | <30s | `time npm run build` |
| Daemon binary size (release) | <10MB | `ls -lh target/release/dyd-daemon` |
| UI bundle size (gzip) | <200KB | `du -sh dist/` |
| Startup (cold) | <300ms (v1), <150ms (target) | tracing spans |
| Idle RSS (daemon) | <15MB | `/proc/pid/status` |
| Idle RSS (UI) | <70MB | Tauri devtools |
| FPS during animation | 60fps (0 dropped in 1000 frames) | Chrome DevTools |
| IPC latency (p95) | <5ms | Built-in histogram |
| Max concurrent plugins | 20 | Load test |
| Plugin startup time | <500ms | Tracing |

---

## Feature Flags

Controlled via `config.json` or environment variables:

| Flag | Config path | Default | Purpose |
|---|---|---|---|
| `DYD_FEATURE_MEDIA` | `media.enabled` | true | Media playback monitoring |
| `DYD_FEATURE_NOTIFICATIONS` | `notifications.enabled` | true | Notification interception |
| `DYD_FEATURE_BATTERY` | `system_status.battery.enabled` | true | Battery monitoring |
| `DYD_FEATURE_WIFI` | `system_status.wifi.enabled` | true | WiFi monitoring |
| `DYD_FEATURE_BLUETOOTH` | `system_status.bluetooth.enabled` | true | Bluetooth monitoring |
| `DYD_FEATURE_VOLUME` | `system_status.volume.enabled` | true | Volume control |
| `DYD_FEATURE_BRIGHTNESS` | `system_status.brightness.enabled` | true | Brightness control |
| `DYD_FEATURE_PLUGINS` | n/a (env: `DYD_PLUGINS_ENABLED`) | true | Plugin system |
| `DYD_FEATURE_AUTO_UPDATE` | n/a (env: `DYD_AUTO_UPDATE`) | true | Auto-update checks |
| `DYD_DEV_MODE` | n/a (env) | false | Dev-only features, verbose logging |
| `DYD_SUPPRESS_NATIVE_NOTIFICATIONS` | `notifications.suppress_native` | true | Suppress GNOME notifications |
| `DYD_REDUCE_MOTION` | `accessibility.reduce_motion` | false | Disable animations |

---

## Release Criteria (v1.0.0)

### Must-have
- [x] Pill visible and interactive on Ubuntu 24.04 + GNOME 46 + Wayland
- [x] Media monitoring works with Spotify and at least one other player
- [x] Play/Pause/Next/Previous controls work
- [x] Notifications are intercepted and displayed
- [x] At least notification action buttons work (reply is optional)
- [x] Battery percentage displays
- [x] Volume slider works
- [x] DND mode toggles
- [x] Animations run at 60fps on integrated Intel GPU
- [x] Startup < 500ms on NVMe SSD
- [x] Idle RAM < 150MB total
- [x] .deb package installs and runs
- [x] AppImage runs
- [x] README with installation instructions
- [x] No known crashes (SEGV, panic)

### Should-have
- [ ] Fullscreen detection works
- [ ] Brightness slider works
- [ ] WiFi indicator works
- [ ] Bluetooth indicator works
- [ ] Notification grouping
- [ ] Notification inline reply
- [ ] Light theme
- [ ] Keyboard shortcuts
- [ ] CI/CD pipeline green

### Nice-to-have (deferred to v1.1)
- [ ] Plugin system
- [ ] Auto-update
- [ ] High contrast mode
- [ ] Scheduled DND
- [ ] Marquee text scrolling
- [ ] Multi-monitor support
- [ ] Player switching (multi-source)
