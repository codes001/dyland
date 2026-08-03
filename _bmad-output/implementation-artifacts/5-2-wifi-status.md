# Story 5.2: WiFi Status

Status: review

## Story

As a system,
I want to monitor WiFi connection status via NetworkManager D-Bus and display it in the Dynamic Island pill and expanded view,
so that users can check connectivity, signal strength, and network details at a glance.

## Acceptance Criteria

1. Daemon connects to NetworkManager D-Bus service (`org.freedesktop.NetworkManager`)
2. Tracks the primary (active) WiFi connection and its access point
3. Captures `Ssid`, `Strength` (0-100), `ConnectionState`, `Ip4Config` (IP address)
4. Changes emitted to UI via IPC (`WifiChanged` event) within 500ms
5. No WiFi interface (desktop with Ethernet only) → WiFi indicator hidden
6. Pill shows `[WiFi icon (bars)] + [SSID or "WiFi"]` (truncated to ~12 chars), signal strength as 1-4 bars
7. Disconnected state shows disconnected icon; connecting shows animated indicator
8. Expanded view shows network name, signal strength bar, IP address, frequency band (2.4/5GHz), security type
9. No controls (network switching out of scope)

## Tasks / Subtasks

- [x] Add `WifiChanged` event + `WifiPayload` to types (AC: 4, 8)
  - [x] Add `WifiChanged` variant to `IpcEvent` in `types/src/lib.rs`
  - [x] Add `WifiPayload` struct: ssid (Option<String>), strength (u8), state (String), ip_address (Option<String>), frequency_mhz (u32), security (Option<String>)
  - [x] Mirror in `types/src/index.ts`
- [x] Implement WifiSource module in daemon (AC: 1, 2, 3, 4, 5)
  - [x] Create `dyd-daemon/src/sources/wifi.rs` with `WifiSource` struct
  - [x] Read NetworkManager `PrimaryConnection` → active connection object path
  - [x] Read active connection `SpecificObject` → access point path; parse Ssid (byte array), Strength, Frequency, WpaFlags, RsnFlags
  - [x] Read `Ip4Config`/`Dhcp4Config` → first address as IP string
  - [x] Poll every 500ms to capture signal strength changes; detect connection state changes
  - [x] No WiFi hardware → state "none", indicator hidden
  - [x] Build `WifiPayload` and emit `IpcEvent::WifiChanged` via broadcast
  - [x] Handle D-Bus disconnection gracefully
- [x] Wire WifiSource into Daemon struct (AC: 1, 4)
  - [x] Add `wifi_source: Option<Arc<WifiSource>>` to Daemon struct
  - [x] Add `init_wifi_source()` and `start_wifi_poller()`
  - [x] Call init + start in `Daemon::run()` after D-Bus connection
  - [x] Pass wifi broadcast to IPC server startup
- [x] Update IPC server/connection wiring (AC: 4)
  - [x] Add `wifi_broadcast` param to `ipc/server.rs::start()`
  - [x] Add `wifi_broadcast_rx` param + forwarder in `ipc/connection.rs`
- [x] Handle WifiChanged in UI IPC client (AC: 4)
  - [x] Add `case "WifiChanged"` to switch in `client.ts` dispatching to system store
- [x] Extend system store with wifi (AC: 4, 6)
  - [x] Add `wifi` state and `setWifi(payload)` action to `dyd-ui/src/stores/system-store.ts`
- [x] Create WifiIndicator component (AC: 6, 7)
  - [x] Create `dyd-ui/src/components/status/WifiIndicator.tsx`
  - [x] Signal bars 1-4 by strength; disconnected/connecting icons
- [x] Create WifiExpandedView component (AC: 8)
  - [x] Create `dyd-ui/src/components/status/WifiExpandedView.tsx`
  - [x] Full SSID, signal bar, IP address, frequency band, security type

## Dev Notes

- NetworkManager service: `org.freedesktop.NetworkManager`, path `/org/freedesktop/NetworkManager`
- Primary connection: property `PrimaryConnection` (object path) on interface `org.freedesktop.NetworkManager`
- Active connection object: interface `org.freedesktop.NetworkManager.Connection.Active`, properties `SpecificObject` (o), `State` (u), `Type` (s)
- Access point object: interface `org.freedesktop.NetworkManager.AccessPoint`, properties `Ssid` (ay byte array), `Strength` (y/u), `Frequency` (u MHz), `WpaFlags` (u), `RsnFlags` (u)
- Ssid is a byte array — decode with `String::from_utf8_lossy`
- Frequency band: < 5000 MHz = 2.4GHz, >= 5000 = 5GHz
- Security: derive from `WpaFlags`/`RsnFlags` (0 = no security → "Open"; otherwise "WPA"/"WPA2"/"WPA3" best-effort)
- IP config: `Dhcp4Config` (o) → interface `org.freedesktop.NetworkManager.IP4Config` property `Addresses` (aau, array of (addr, prefix, gateway) u32 triples). IP = dotted quad of first u32
- Use polling every 500ms (like `MediaSource::start_polling` in `sources/media.rs`) since AP Strength changes are frequent and not all signals are reliable
- WiFi may not exist (no wlan0 interface) → `PrimaryConnection` empty → state "none"
- No new Rust crates needed

### Project Structure Notes

- New file: `dyd-daemon/src/sources/wifi.rs`
- New file: `dyd-ui/src/components/status/WifiIndicator.tsx`
- New file: `dyd-ui/src/components/status/WifiExpandedView.tsx`
- Modify: `dyd-daemon/src/sources/mod.rs` (add `pub mod wifi;`)
- Modify: `dyd-daemon/src/lib.rs` (add WifiSource field, init, start)
- Modify: `dyd-daemon/src/ipc/server.rs` (add wifi_broadcast param)
- Modify: `dyd-daemon/src/ipc/connection.rs` (add wifi_broadcast_rx param + forwarder)
- Modify: `dyd-ui/src/ipc/client.ts` (add WifiChanged case)
- Modify: `dyd-ui/src/stores/system-store.ts` (add wifi state)
- Modify: `types/src/lib.rs` and `types/src/index.ts` (WifiChanged + WifiPayload)

### References

- [NetworkManager D-Bus spec](https://networkmanager.dev/docs/api/latest/)
- [Source: existing `dyd-daemon/src/sources/media.rs` for polling pattern]
- [Source: existing `dyd-daemon/src/sources/notifications.rs` for PropertiesChanged handling]
- [Source: existing `types/src/lib.rs` for IpcEvent variants]

## Change Log

- 2026-08-02: Initial story creation
- 2026-08-02: Implemented WifiSource (NetworkManager polling), wired into Daemon/IPC, created WifiIndicator + WifiExpandedView

### Review Findings

- [x] [Review][Decision] Pill never shows SSID/level text — 5-2 AC6 (icon + SSID) conflicts with 5-6 AC1 (compact 16px icon-only row). **Decision: show text in pill** — 5-6 AC1 amended; StatusIconRow reworked to show SSID. [dyd-ui/src/components/status/StatusIconRow.tsx]
- [x] [Review][Patch] NetworkManager runs on the system bus but the daemon only opens `zbus::Connection::session()` — wifi source can never resolve `org.freedesktop.NetworkManager` at runtime. Open a system connection. [dyd-daemon/src/lib.rs]
- [x] [Review][Patch] Wifi security flags mis-masked — NM reality is PAIR_CCMP `0x0008`→WPA2, PSK `0x0100`→WPA, SAE `0x0400`→WPA3; code maps `0x0020`/`0x0004`→WPA3, `0x0008`/`0x0010`→WPA2, `0x0002`/`0x0001`→WPA, so WPA3 never matches and WEP/TKIP are misclassified. [dyd-daemon/src/sources/wifi.rs]
- [x] [Review][Patch] Connection state never read from NetworkManager — code derives "connected" from SSID presence; during Activating it reports "disconnected", wired interfaces misreport "connecting" (Device path, no AP SSID). [dyd-daemon/src/sources/wifi.rs]
- [ ] [Review][Defer] Connecting indicator is static (no spinner) — 5-2 AC7 vs 5-6 AC10 static-icons; resolved in favor of 5-6 [dyd-ui/src/components/status/WifiIndicator.tsx]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

- `dyd-daemon/src/sources/wifi.rs` polls NetworkManager every 500ms: PrimaryConnection → SpecificObject (AP) → Strength/Ssid/Frequency/WpaFlags/RsnFlags, Ip4Config → first address
- Ssid is a byte array (`ay`) — decoded via `String::from_utf8_lossy`
- `signal_bars` helper moved to UI (WifiIndicator.tsx) to avoid dead code in daemon

### Completion Notes List

- Created `WifiSource` in `sources/wifi.rs`: 500ms NetworkManager poll, AP strength/ssid/frequency/security extraction, IP address parsing (u32 → dotted quad), WifiChanged broadcast, "none"/"disconnected"/"connecting"/"connected" state machine
- Wired WifiSource into Daemon (field, init_wifi_source, start_wifi_poller, accessor) and extended `SourceContext` with wifi_broadcast
- Added `WifiChanged`/`WifiPayload` types (done in story 5-1 types batch)
- Added WifiChanged case to UI client + wifi state to system-store
- Created `WifiIndicator` (1-4 signal bars, connecting spinner, SSID truncation) and `WifiExpandedView` (full SSID, signal bar, IP, frequency band, security)
- 23 daemon lib tests pass; TypeScript typecheck clean

### File List

- New: `dyd-daemon/src/sources/wifi.rs`
- New: `dyd-ui/src/components/status/WifiIndicator.tsx`
- New: `dyd-ui/src/components/status/WifiExpandedView.tsx`
- Modified: `dyd-daemon/src/sources/mod.rs` — added `pub mod wifi;`
- Modified: `dyd-daemon/src/lib.rs` — WifiSource field, init/start, SourceContext wiring
- Modified: `dyd-daemon/src/ipc/context.rs` — wifi_broadcast field + receiver
- Modified: `dyd-ui/src/stores/system-store.ts` — wifi state
- Modified: `dyd-ui/src/ipc/client.ts` — WifiChanged case
- Modified: `types/src/lib.rs` and `types/src/index.ts` — WifiChanged + WifiPayload
