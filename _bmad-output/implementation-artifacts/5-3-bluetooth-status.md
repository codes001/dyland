# Story 5.3: Bluetooth Status

Status: review

## Story

As a system,
I want to monitor Bluetooth status via BlueZ D-Bus and display it in the Dynamic Island pill and expanded view,
so that users know if Bluetooth is active and what devices are connected.

## Acceptance Criteria

1. Daemon connects to BlueZ D-Bus service (`org.bluez`)
2. Monitors adapter (`/org/bluez/hci0`) `Powered` and `Discoverable` properties
3. Monitors connected devices via `/org/bluez/hci0/dev_*` paths, capturing name + connected state
4. Changes emitted to UI via IPC (`BluetoothChanged` event) within 500ms
5. No Bluetooth hardware → indicator hidden
6. Pill shows `[Bluetooth icon] + [device count if > 0]`; icon solid when on, outlined when off
7. Expanded view shows Bluetooth on/off toggle + list of connected devices (name + type icon)
8. `SetBluetoothPowered` command sets adapter `Powered` property (interactive toggle)

## Tasks / Subtasks

- [x] Add `BluetoothChanged` event + `BluetoothPayload` to types (AC: 4, 6)
  - [x] Add `BluetoothChanged` variant to `IpcEvent` in `types/src/lib.rs`
  - [x] Add `BluetoothPayload` struct: powered (bool), devices (Vec<BluetoothDevice>)
  - [x] Add `BluetoothDevice` struct: name (String), connected (bool), icon (Option<String>), address (String)
  - [x] Mirror in `types/src/index.ts`
- [x] Implement BluetoothSource module in daemon (AC: 1, 2, 3, 4, 5)
  - [x] Create `dyd-daemon/src/sources/bluetooth.rs` with `BluetoothSource` struct
  - [x] Detect adapter via `org.freedesktop.DBus.ObjectManager.GetManagedObjects` on org.bluez
  - [x] Read adapter `Powered` property (interface `org.bluez.Adapter1`)
  - [x] Enumerate `/org/bluez/hci0/dev_*` device objects, read `Name`, `Connected`, `Icon` (interface `org.bluez.Device1`)
  - [x] Poll every 500ms to capture adapter/device changes
  - [x] No adapter → state "none", indicator hidden
  - [x] Build `BluetoothPayload` and emit `IpcEvent::BluetoothChanged` via broadcast
  - [x] Implement `set_powered(powered)` → write adapter `Powered` property via `Properties.Set`
  - [x] Handle D-Bus disconnection gracefully
- [x] Wire BluetoothSource into Daemon struct (AC: 1, 4)
  - [x] Add `bluetooth_source: Option<Arc<BluetoothSource>>` to Daemon struct
  - [x] Add `init_bluetooth_source()` and `start_bluetooth_poller()`
  - [x] Call init + start in `Daemon::run()` after D-Bus connection
  - [x] Pass bluetooth broadcast + source to IPC server startup
- [x] Update IPC server/connection wiring (AC: 4)
  - [x] Add `bluetooth_broadcast` param to `ipc/server.rs::start()`
  - [x] Add `bluetooth_broadcast_rx` param + forwarder in `ipc/connection.rs`
  - [x] Replace `SetBluetoothPowered` stub in `connection.rs:287` with real source call
- [x] Handle BluetoothChanged in UI IPC client (AC: 4)
  - [x] Add `case "BluetoothChanged"` to switch in `client.ts` dispatching to system store
- [x] Extend system store with bluetooth (AC: 4, 6)
  - [x] Add `bluetooth` state and `setBluetooth(payload)` action to `dyd-ui/src/stores/system-store.ts`
- [x] Create BluetoothIndicator component (AC: 6)
  - [x] Create `dyd-ui/src/components/status/BluetoothIndicator.tsx`
  - [x] Solid icon when powered + device count badge; outlined when off
- [x] Create BluetoothExpandedView component (AC: 7)
  - [x] Create `dyd-ui/src/components/status/BluetoothExpandedView.tsx`
  - [x] On/off toggle sending `SetBluetoothPowered`; list of connected devices; "No devices connected" when empty

## Dev Notes

- BlueZ service: `org.bluez`, root path `/`
- Adapter path: `/org/bluez/hci0`, interface `org.bluez.Adapter1`, properties `Powered` (b), `Discoverable` (b)
- Device paths: `/org/bluez/hci0/dev_XX_XX_XX_XX_XX_XX`, interface `org.bluez.Device1`, properties `Name` (s), `Connected` (b), `Icon` (s), `Address` (s), `Class` (u)
- Use `org.freedesktop.DBus.ObjectManager.GetManagedObjects` on `org.bluez` `/` to list all objects at once, then filter by path prefix and interface
- Device type icon: use `Icon` property if present, else derive from `Class`/`Appearance`; fallback "computer"
- Set Powered: `conn.call_method(Some("org.bluez"), "/org/bluez/hci0", Some("org.freedesktop.DBus.Properties"), "Set", &("org.bluez.Adapter1", "Powered", &zvariant::Value::Bool(powered)))`
- `SetBluetoothPowered` command type already exists; only the connection.rs handler is a stub
- Poll every 500ms (same pattern as `MediaSource::start_polling`)
- No new Rust crates needed

### Project Structure Notes

- New file: `dyd-daemon/src/sources/bluetooth.rs`
- New file: `dyd-ui/src/components/status/BluetoothIndicator.tsx`
- New file: `dyd-ui/src/components/status/BluetoothExpandedView.tsx`
- Modify: `dyd-daemon/src/sources/mod.rs` (add `pub mod bluetooth;`)
- Modify: `dyd-daemon/src/lib.rs` (add BluetoothSource field, init, start)
- Modify: `dyd-daemon/src/ipc/server.rs` (add bluetooth_broadcast + bluetooth_source params)
- Modify: `dyd-daemon/src/ipc/connection.rs` (add bluetooth_broadcast_rx param + forwarder, real SetBluetoothPowered)
- Modify: `dyd-ui/src/ipc/client.ts` (add BluetoothChanged case)
- Modify: `dyd-ui/src/stores/system-store.ts` (add bluetooth state)
- Modify: `types/src/lib.rs` and `types/src/index.ts` (BluetoothChanged + BluetoothPayload + BluetoothDevice)

### References

- [BlueZ D-Bus API](https://github.com/bluez/bluez/blob/master/doc/org.bluez.Adapter.rst)
- [Source: existing `dyd-daemon/src/sources/media.rs` for polling pattern]
- [Source: existing `dyd-daemon/src/sources/notifications.rs` for D-Bus method calls]
- [Source: existing `types/src/lib.rs` for IpcEvent variants]

## Change Log

- 2026-08-02: Initial story creation
- 2026-08-02: Implemented BluetoothSource (BlueZ GetManagedObjects poll, set_powered), wired into Daemon/SourceContext/IPC, SetBluetoothPowered real handler, BluetoothIndicator + BluetoothExpandedView UI, partial_eq derives on BluetoothDevice/BluetoothPayload. Fixed zvariant 4.2 downcast owned-return + body temporary-lifetime issues.

### Review Findings

- [x] [Review][Decision] Device count never shown in pill — 5-3 AC6 (icon + count) conflicts with 5-6 AC1 (compact 16px icon-only row). **Decision: show text in pill** — 5-6 AC1 amended; StatusIconRow reworked to show device count. [dyd-ui/src/components/status/StatusIconRow.tsx]
- [x] [Review][Patch] BlueZ runs on the system bus but the daemon only opens `zbus::Connection::session()` — bluetooth source can never resolve `org.bluez` at runtime. Open a system connection. [dyd-daemon/src/lib.rs]
- [x] [Review][Dismiss] Bluetooth downcast_ref "compile errors" (Blind Hunter) — disproven; `cargo build -p dyd-daemon` clean, 30 tests pass.
- [x] [Review][Patch] Multi-adapter nondeterminism — powered state and connected-device lookup pick an arbitrary adapter from the Map; should sort and use a deterministic adapter (e.g. first hci). [dyd-daemon/src/sources/bluetooth.rs]
- [x] [Review][Patch] Device list ordering nondeterministic (HashMap iteration) → spurious broadcasts on every poll. Sort devices by alias/address. [dyd-daemon/src/sources/bluetooth.rs]
- [x] [Review][Patch] AC2 missing: Discoverable (paired/trusted discovery) never monitored — payload has no discovery signal at all. [dyd-daemon/src/sources/bluetooth.rs]
- [x] [Review][Patch] AC5 deviation: adapter absence not propagated — payload has no "present" sentinel, so UI can't distinguish "no adapter" from "off". [dyd-daemon/src/sources/bluetooth.rs]
- [x] [Review][Patch] SetBluetoothPowered returns ok even when no source exists (unreachable arm). [dyd-daemon/src/ipc/connection.rs]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- zvariant 4.2 `Value::downcast_ref<T>` returns owned `T` (Result<T>), so `.copied()`/`.cloned()` on `Option<bool>`/`Option<String>` failed with E0599/E0614 — removed.
- `body().deserialize::<HashMap<String, ... zvariant::Value>>()` on a temporary returned E0716 (Value borrows the body) — bound `msg.body()` to a local first.
- `Vec<BluetoothDevice>` needed `PartialEq` (BluetoothState derives PartialEq) — added derive in `types/src/lib.rs`.

### File List

- types/src/lib.rs (BluetoothChanged already present; added PartialEq derives)
- types/src/index.ts (already present)
- dyd-daemon/src/sources/bluetooth.rs (new)
- dyd-daemon/src/sources/mod.rs
- dyd-daemon/src/lib.rs
- dyd-daemon/src/ipc/context.rs
- dyd-daemon/src/ipc/server.rs
- dyd-daemon/src/ipc/connection.rs
- dyd-ui/src/ipc/client.ts
- dyd-ui/src/stores/system-store.ts
- dyd-ui/src/components/status/BluetoothIndicator.tsx (new)
- dyd-ui/src/components/status/BluetoothExpandedView.tsx (new)
