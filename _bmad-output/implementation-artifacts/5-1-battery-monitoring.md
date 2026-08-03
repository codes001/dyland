# Story 5.1: Battery Monitoring

Status: review

## Story

As a system,
I want to monitor battery status via UPower D-Bus and display it in the Dynamic Island pill and expanded view,
so that users can see battery percentage, charging state, and time remaining at a glance.

## Acceptance Criteria

1. Daemon connects to UPower D-Bus service (`org.freedesktop.UPower`) and subscribes to `PropertiesChanged` on the primary battery device (`/org/freedesktop/UPower/devices/DisplayDevice`)
2. Captures `Percentage`, `State` (charging/discharging/full), `TimeToEmpty`, `TimeToFull`, `IsPresent`
3. Changes are emitted to UI via IPC (`BatteryChanged` event) within 200ms
4. No battery present (desktop) → `IsPresent: false` → battery indicator hidden
5. Pill shows `[battery icon] + [percentage text]` (e.g., "85%")
6. Icon changes based on state: charging (bolt), discharging (standard), full (checkmark), low <20% (red), critical <10% (red + pulsing)
7. Expanded view shows large battery icon, percentage, time remaining ("2h 15m remaining" / "1h 30m until full"), and battery health (if available)
8. No polling — event-driven via UPower signals only

## Tasks / Subtasks

- [x] Add `BatteryChanged` handling in UI IPC client (AC: 3)
  - [x] Add `case "BatteryChanged"` to switch in `client.ts` dispatching to system store
- [x] Create battery Zustand store (AC: 3, 5)
  - [x] Create `dyd-ui/src/stores/system-store.ts` with `battery` state and `setBattery(payload)` action
- [x] Implement BatterySource module in daemon (AC: 1, 2, 3, 8)
  - [x] Create `dyd-daemon/src/sources/battery.rs` with `BatterySource` struct following `MediaSource`/`NotificationSource` pattern
  - [x] Add D-Bus `AddMatch` for `PropertiesChanged` on `/org/freedesktop/UPower/devices/DisplayDevice`
  - [x] Use `MessageStream` to receive PropertiesChanged signals
  - [x] Fetch initial battery state via `Properties.Get` (Percentage, State, TimeToEmpty, TimeToFull, IsPresent)
  - [x] Parse `State` enum: 0=unknown, 1=charging, 2=discharging, 3=empty, 4=fully charged, 5=pending charge, 6=pending discharge
  - [x] Build `BatteryPayload` and emit `IpcEvent::BatteryChanged` via broadcast
  - [x] Handle D-Bus disconnection gracefully
- [x] Wire BatterySource into Daemon struct (AC: 1, 3)
  - [x] Add `battery_source: Option<Arc<BatterySource>>` to Daemon struct
  - [x] Add `init_battery_source()` and `start_battery_listener()`
  - [x] Call init + start in `Daemon::run()` after D-Bus connection
  - [x] Pass battery broadcast to IPC server startup
- [x] Update IPC server/connection wiring (AC: 3)
  - [x] Add `battery_broadcast` param to `ipc/server.rs::start()`
  - [x] Add `battery_broadcast_rx` param + forwarder in `ipc/connection.rs`
- [x] Create BatteryIndicator component (AC: 5, 6)
  - [x] Create `dyd-ui/src/components/status/BatteryIndicator.tsx`
  - [x] Show icon + percentage; state-based icons (charging bolt, low red, critical pulsing)
- [x] Create BatteryExpandedView component (AC: 7)
  - [x] Create `dyd-ui/src/components/status/BatteryExpandedView.tsx`
  - [x] Large percentage, time remaining formatting ("2h 15m remaining"), battery health
- [x] Update types (AC: 7)
  - [x] Add `health: Option<f64>` field to `BatteryPayload` in Rust/TS

## Dev Notes

- UPower service: `org.freedesktop.UPower`, device path: `/org/freedesktop/UPower/devices/DisplayDevice`, interface: `org.freedesktop.UPower.Device`
- Battery property names: `Percentage` (f64), `State` (u32), `TimeToEmpty` (i64 seconds), `TimeToFull` (i64 seconds), `IsPresent` (bool)
- DisplayDevice aggregates multiple physical batteries — ideal for laptops with multiple batteries
- `BatteryPayload` and `IpcEvent::BatteryChanged` ALREADY exist in `types/src/lib.rs` and `types/src/index.ts` — reuse them, only add `health` field
- Follow the `NotificationSource` pattern in `sources/notifications.rs` (AddMatch + MessageStream + filter by interface/member)
- Do NOT poll — UPower emits PropertiesChanged on state changes (event-driven)
- State enum values: 0=unknown, 1=charging, 2=discharging, 3=empty, 4=fully charged, 5=pending charge, 6=pending discharge
- The `IpcCommand::SetVolume/SetMute/SetBrightness/SetBluetoothPowered` and event stubs already exist in `connection.rs:277-291`
- Broadcast fan-out uses the existing `spawn_broadcast_forwarder` helper in `connection.rs:29`
- No new Rust crates needed — zbus, tokio, futures already available
- Time formatting: seconds → "Xh Ym" or "Xm" under 1 hour
- Battery health: `EnergyFullDesign` vs `EnergyFull` ratio on DisplayDevice

### Project Structure Notes

- New file: `dyd-daemon/src/sources/battery.rs`
- New file: `dyd-ui/src/stores/system-store.ts`
- New file: `dyd-ui/src/components/status/BatteryIndicator.tsx`
- New file: `dyd-ui/src/components/status/BatteryExpandedView.tsx`
- Modify: `dyd-daemon/src/sources/mod.rs` (add `pub mod battery;`)
- Modify: `dyd-daemon/src/lib.rs` (add BatterySource field, init, start)
- Modify: `dyd-daemon/src/ipc/server.rs` (add battery_broadcast param)
- Modify: `dyd-daemon/src/ipc/connection.rs` (add battery_broadcast_rx param + forwarder)
- Modify: `dyd-ui/src/ipc/client.ts` (add BatteryChanged case)
- Modify: `types/src/lib.rs` and `types/src/index.ts` (add health field to BatteryPayload)

### References

- [UPower D-Bus spec](https://upower.freedesktop.org/docs/Device.html)
- [Source: existing `dyd-daemon/src/sources/notifications.rs` for AddMatch + MessageStream pattern]
- [Source: existing `dyd-daemon/src/lib.rs` for Daemon struct and source initialization]
- [Source: existing `types/src/lib.rs` for BatteryPayload, IpcEvent::BatteryChanged]
- [Source: existing `dyd-daemon/src/ipc/connection.rs` for spawn_broadcast_forwarder helper]
- [Source: existing `dyd-ui/src/stores/notification-store.ts` for Zustand store pattern]

## Change Log

- 2026-08-02: Initial story creation
- 2026-08-02: Implemented BatterySource (UPower AddMatch + MessageStream), wired into Daemon/IPC, created system-store + BatteryIndicator + BatteryExpandedView, added health field to BatteryPayload
- 2026-08-02: Code review (Epic 5) — findings recorded below

### Review Findings

- [x] [Review][Decision] Pill never shows battery percentage — 5-1 AC5 (icon + %) conflicts with 5-6 AC1 (compact 16px icon-only row). **Decision: show text in pill** — 5-6 AC1 amended; StatusIconRow reworked to show battery %. [dyd-ui/src/components/status/StatusIconRow.tsx]
- [x] [Review][Patch] UPower runs on the system bus but the daemon only opens `zbus::Connection::session()` — battery source can never resolve `org.freedesktop.UPower` at runtime. Also affects wifi (NetworkManager) + bluetooth (BlueZ). Open a system connection for these sources. [dyd-daemon/src/lib.rs]
- [x] [Review][Patch] Battery listener has no recovery loop — if UPower AddMatch/initial fetch fails transiently, the listener exits and battery state is never populated again. [dyd-daemon/src/sources/battery.rs]
- [x] [Review][Patch] Transient read failure flips `is_present` to false — a momentary Percentage Get failure (incl. mid-signal) hides the battery indicator until a later success. [dyd-daemon/src/sources/battery.rs]
- [x] [Review][Patch] Health ratio unclamped — EnergyFull/EnergyFullDesign can exceed 100% on aged batteries, rendering "Health 115%". [dyd-daemon/src/sources/battery.rs]
- [x] [Review][Patch] Discharging ≥99.5% shows contradictory "Fully Charged" + "Xm remaining". [dyd-ui/src/components/status/BatteryExpandedView.tsx]
- [x] [Review][Patch] AC6 color deviation — spec requires red for <20%; code uses orange 10-20% (red only <10%). [dyd-ui/src/components/status/BatteryIndicator.tsx]
- [x] [Review][Defer] Health can show "Health 0%" when EnergyFull=0 — acceptable edge, not a hard violation [dyd-daemon/src/sources/battery.rs] — deferred, acceptable edge

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

- `dyd-daemon/src/sources/battery.rs` uses AddMatch for PropertiesChanged on `/org/freedesktop/UPower/devices/DisplayDevice` + MessageStream listener
- Refactored IPC server/connection to use shared `SourceContext` struct (`ipc/context.rs`) to absorb growing source params
- Battery health computed from EnergyFull / EnergyFullDesign ratio

### Completion Notes List

- Created `BatterySource` in `sources/battery.rs`: D-Bus AddMatch on UPower DisplayDevice PropertiesChanged, MessageStream listener, initial Properties.Get fetch, state parsing (UPower state enum), BatteryChanged broadcast emission, battery health via EnergyFull/EnergyFullDesign ratio
- Wired BatterySource into Daemon: field, init_battery_source, start_battery_listener, run() integration, source_ref accessor
- Refactored IPC wiring: new `SourceContext` struct bundles all source broadcasts + Arc refs; `ipc/server.rs::start()` and `connection::handle_connection()` now take `SourceContext` — eliminates too_many_arguments, easily extensible for remaining system sources
- Added `health: Option<f64>` to `BatteryPayload` (Rust + TS)
- Added WifiChanged/BluetoothChanged/VolumeChanged/BrightnessChanged events + payloads to types (Rust + TS) for upcoming stories
- Created `system-store.ts` Zustand store with battery/wifi/bluetooth/volume/brightness state + selectedStatusFeature
- Added BatteryChanged/WifiChanged/BluetoothChanged/VolumeChanged/BrightnessChanged cases to UI IPC client
- Created `BatteryIndicator` (icon + %, charging bolt, low/critical colors, pulse animation) and `BatteryExpandedView` (large %, time formatting, health badge)
- All 21 daemon lib tests pass; TypeScript typecheck clean

### File List

- New: `dyd-daemon/src/sources/battery.rs`
- New: `dyd-daemon/src/ipc/context.rs`
- New: `dyd-ui/src/stores/system-store.ts`
- New: `dyd-ui/src/components/status/BatteryIndicator.tsx`
- New: `dyd-ui/src/components/status/BatteryExpandedView.tsx`
- Modified: `dyd-daemon/src/sources/mod.rs` — added `pub mod battery;`
- Modified: `dyd-daemon/src/lib.rs` — BatterySource field, init/start, SourceContext wiring
- Modified: `dyd-daemon/src/ipc/mod.rs` — added `pub mod context;`
- Modified: `dyd-daemon/src/ipc/server.rs` — uses SourceContext
- Modified: `dyd-daemon/src/ipc/connection.rs` — uses SourceContext
- Modified: `dyd-ui/src/ipc/client.ts` — system status event cases
- Modified: `dyd-ui/src/index.css` — battery pulse keyframes
- Modified: `types/src/lib.rs` — health field, new system payloads/events
- Modified: `types/src/index.ts` — health field, new system payloads/events
