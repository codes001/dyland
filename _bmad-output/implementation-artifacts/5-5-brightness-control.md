# Story 5.5: Brightness Control

Status: review

## Story

As a system,
I want to monitor and control screen brightness via sysfs backlight (fallback GNOME SettingsDaemon D-Bus),
so that users can see brightness level in the pill and adjust it with a slider in the expanded view.

## Acceptance Criteria

1. Daemon detects backlight devices via `/sys/class/backlight/*` existence
2. Reads current brightness (0-100) and max brightness from sysfs
3. Optionally monitors GNOME SettingsDaemon Power D-Bus (`org.gnome.SettingsDaemon.Power`) `Brightness` property
4. Changes emitted to UI via IPC (`BrightnessChanged` event) within 200ms
5. No backlight device (desktop monitor) → brightness indicator hidden
6. Pill shows `[sun icon]` with icon brightness corresponding to level (opacity change)
7. Expanded view shows brightness slider (0-100%, draggable) that adjusts in real-time
8. `SetBrightness` command writes to `/sys/class/backlight/*/brightness` (fallback: GNOME SettingsDaemon `SetBrightness`)

## Tasks / Subtasks

- [x] Add `BrightnessChanged` event + `BrightnessPayload` to types (AC: 4, 5)
  - [x] Add `BrightnessChanged` variant to `IpcEvent` in `types/src/lib.rs`
  - [x] Add `BrightnessPayload` struct: brightness (u8), has_backlight (bool)
  - [x] Mirror in `types/src/index.ts`
- [x] Implement BrightnessSource module in daemon (AC: 1, 2, 3, 4, 5, 8)
  - [x] Create `dyd-daemon/src/sources/brightness.rs` with `BrightnessSource` struct
  - [x] Detect backlight via `std::fs::read_dir("/sys/class/backlight")`; filter dirs with `brightness` + `max_brightness` files
  - [x] Read `brightness` and `max_brightness` → compute percentage 0-100
  - [x] Poll every 200ms to detect changes
  - [x] No backlight → has_backlight false, no emission
  - [x] Build `BrightnessPayload` and emit `IpcEvent::BrightnessChanged` via broadcast
  - [x] Implement `set_brightness(u8)` → write `(value/100 * max)` to sysfs `brightness` file
  - [x] Try GNOME SettingsDaemon D-Bus `Brightness` property as secondary source (best-effort)
  - [x] Handle sysfs errors gracefully (permission denied logged, no crash)
- [x] Wire BrightnessSource into Daemon struct (AC: 1, 4)
  - [x] Add `brightness_source: Option<Arc<BrightnessSource>>` to Daemon struct
  - [x] Add `init_brightness_source()` and `start_brightness_poller()`
  - [x] Call init + start in `Daemon::run()`
  - [x] Pass brightness broadcast + source to IPC server startup
- [x] Update IPC server/connection wiring (AC: 8)
  - [x] Add `brightness_broadcast` param to `ipc/server.rs::start()`
  - [x] Add `brightness_broadcast_rx` param + forwarder in `ipc/connection.rs`
  - [x] Replace `SetBrightness` stub in `connection.rs:282` with real source call
- [x] Handle BrightnessChanged in UI IPC client (AC: 4)
  - [x] Add `case "BrightnessChanged"` to switch in `client.ts` dispatching to system store
- [x] Extend system store with brightness (AC: 4, 6)
  - [x] Add `brightness` state and `setBrightness(payload)` action to `dyd-ui/src/stores/system-store.ts`
- [x] Create BrightnessIndicator component (AC: 6)
  - [x] Create `dyd-ui/src/components/status/BrightnessIndicator.tsx`
  - [x] Sun icon with opacity corresponding to level
- [x] Create BrightnessExpandedView component (AC: 7, 8)
  - [x] Create `dyd-ui/src/components/status/BrightnessExpandedView.tsx`
  - [x] Range slider (0-100%) sending `SetBrightness`

## Dev Notes

- Backlight devices: `/sys/class/backlight/<device>/brightness` and `/sys/class/backlight/<device>/max_brightness`
- brightness value must be clamped to [0, max_brightness]
- Write to sysfs may require permissions; log warnings on failure and continue
- GNOME SettingsDaemon Power D-Bus: service `org.gnome.SettingsDaemon.Power`, path `/org/gnome/SettingsDaemon/Power`, interface `org.gnome.SettingsDaemon.Power.Screen`, property `Brightness` (i, 0-100); method `SetBrightness` (i)
- Prefer sysfs for both read and write (simplest, works on most laptops). D-Bus is best-effort secondary
- Use `tokio::fs` for async file reads/writes (or std fs wrapped in spawn_blocking; std fs is fine for 200ms polling)
- Poll every 200ms to satisfy the 200ms emission AC
- Brightness at 0% (screen off on some systems) → handle gracefully, still emit
- No new Rust crates needed

### Project Structure Notes

- New file: `dyd-daemon/src/sources/brightness.rs`
- New file: `dyd-ui/src/components/status/BrightnessIndicator.tsx`
- New file: `dyd-ui/src/components/status/BrightnessExpandedView.tsx`
- Modify: `dyd-daemon/src/sources/mod.rs` (add `pub mod brightness;`)
- Modify: `dyd-daemon/src/lib.rs` (add BrightnessSource field, init, start)
- Modify: `dyd-daemon/src/ipc/server.rs` (add brightness_broadcast + brightness_source params)
- Modify: `dyd-daemon/src/ipc/connection.rs` (add brightness_broadcast_rx param + forwarder, real SetBrightness)
- Modify: `dyd-ui/src/ipc/client.ts` (add BrightnessChanged case)
- Modify: `dyd-ui/src/stores/system-store.ts` (add brightness state)
- Modify: `types/src/lib.rs` and `types/src/index.ts` (BrightnessChanged + BrightnessPayload)

### References

- [Backlight sysfs ABI](https://www.kernel.org/doc/Documentation/ABI/stable/sysfs-class-backlight)
- [Source: existing `dyd-daemon/src/sources/media.rs` for polling pattern]
- [Source: existing `dyd-daemon/src/sources/fullscreen.rs` for sysfs/DBus interaction]
- [Source: existing `types/src/lib.rs` for IpcEvent variants]

## Change Log

- 2026-08-02: Initial story creation
- 2026-08-02: Implemented BrightnessSource (sysfs backlight poll + GNOME SettingsDaemon D-Bus fallback), wired into Daemon/SourceContext/IPC, real SetBrightness handler, BrightnessIndicator + BrightnessExpandedView UI.

### Review Findings

- [x] [Review][Patch] set_brightness never falls back to D-Bus on sysfs write failure — `std::fs::write(...)?` propagates the error, so the D-Bus SetBrightness branch is only reachable when no sysfs device exists. [dyd-daemon/src/sources/brightness.rs]
- [x] [Review][Patch] `max_brightness == 0` still yields `has_backlight: true` (guarded read leaves max at 0). Treat max==0 as no-backlight. [dyd-daemon/src/sources/brightness.rs]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- sysfs read via tokio::fs, write via std::fs (200ms polling acceptable per story notes).
- GNOME SettingsDaemon D-Bus used as fallback source only when no sysfs backlight device; reads `Brightness` property (i32) and writes via `SetBrightness`.
- `msg.body().deserialize::<zvariant::Value>()` borrows the temporary body (E0716/E0597) — deserialized directly to owned `i32` instead.
- 30 lib tests pass; typecheck clean.

### File List

- dyd-daemon/src/sources/brightness.rs (new)
- dyd-daemon/src/sources/mod.rs
- dyd-daemon/src/lib.rs
- dyd-daemon/src/ipc/context.rs
- dyd-daemon/src/ipc/connection.rs
- dyd-ui/src/components/status/BrightnessIndicator.tsx (new)
- dyd-ui/src/components/status/BrightnessExpandedView.tsx (new)
- types/src/lib.rs, types/src/index.ts (already present)
