# Story 4.1: Notification Source (Daemon-side)

Status: review

## Story

As a system,
I want the daemon to intercept notifications via D-Bus (org.freedesktop.Notifications),
so that notification data can be forwarded to the UI for display in the Dynamic Island.

## Acceptance Criteria

1. Daemon connects to the session D-Bus and subscribes to `org.freedesktop.Notifications` signals
2. On incoming `Notify` signal, extracts: app_name, summary, body, urgency, notification ID
3. Sends `NotificationReceived` IPC event via broadcast when a new notification arrives
4. Daemon handles `NotificationAction` IPC command by calling `org.freedesktop.Notifications.InvokeAction` on session bus (if supported) or logs the action
5. Daemon handles `NotificationClose` IPC command by calling `org.freedesktop.Notifications.CloseNotification` with the notification ID
6. UI receives `NotificationReceived` events and stores the latest notification in a Zustand store
7. Gracefully handles D-Bus disconnection — reconnects on next signal
8. Logs notification events at debug level, first connection at info level

## Tasks / Subtasks

- [x] Implement NotificationSource module in daemon (AC: 1, 2, 3, 7)
  - [x] Create `dyd-daemon/src/sources/notifications.rs` with `NotificationSource` struct
  - [x] Subscribe to `org.freedesktop.Notifications` by monitoring `Notify` method calls via D-Bus `AddMatch` with eavesdrop
  - [x] Extract payload: app_name, summary, body, urgency (from hints dict), notification ID
  - [x] Build `NotificationReceived` IPC event with `NotificationPayload`
  - [x] Send via broadcast channel (daemon -> IPC -> Tauri -> UI)
  - [x] Use `zbus::Connection::session()` — already connected in Daemon
  - [x] Handle D-Bus disconnection: log warning, retry on next event
- [x] Wire into Daemon struct (AC: 7, 8)
  - [x] Add `notification_source: Option<Arc<NotificationSource>>` to Daemon struct
  - [x] Add `init_notification_source()`: create NotificationSource, set D-Bus connection
  - [x] Add `start_notification_listener()`: spawn background task with shutdown signal
  - [x] Add `notification_source_ref()` accessor
  - [x] Call init + start in `Daemon::run()` after D-Bus connection
  - [x] Pass notification broadcast + source to IPC server startup
  - [x] Log notification source initialization at info level
- [x] Update IPC connection handler (AC: 4, 5)
  - [x] In `connection.rs`, replace `NotificationAction` stub with real implementation:
    - [x] Spawn task to call `org.freedesktop.Notifications.InvokeAction` if supported
    - [x] Log action requests at debug level
  - [x] Replace `NotificationClose` stub with real implementation:
    - [x] Spawn task to call `org.freedesktop.Notifications.CloseNotification(id)`
    - [x] Log close requests at debug level
  - [x] Pass `NotificationSource` reference to connection handler
- [x] Handle NotificationReceived in UI IPC client (AC: 6)
  - [x] Add `case "NotificationReceived"` to switch in `client.ts`
  - [x] Dispatch payload to `useNotificationStore.getState().setNotification()`
- [x] Create notification Zustand store (AC: 6)
  - [x] Create `dyd-ui/src/stores/notification-store.ts`
  - [x] Define `NotificationState`: id, appName, appIcon, summary, body, urgency, timestamp
  - [x] Implement `setNotification(payload)` action
  - [x] Implement `clearNotification()` action
- [x] Update types (if needed) (AC: 4, 5)
  - [x] Add `actions: Vec<NotificationActionItem>` to `NotificationPayload` in Rust/TS
  - [x] Ensure `NotificationAction`/`NotificationClose` command types match daemon expectations

## Dev Notes

- Use `zbus::Connection::match_rule()` to subscribe to `org.freedesktop.Notifications` signal `Notify`
- Signal `Notify` is on interface `org.freedesktop.Notifications`, path `/org/freedesktop/Notifications`
- Notification urgency comes from the hints dict: `urgency` key (0=low, 1=normal, 2=critical)
- Already existing in types: `NotificationPayload`, `NotificationReceived` event, `NotificationAction`/`NotificationClose` commands
- `NotificationAction` and `NotificationClose` are currently stubs in `connection.rs:236-240`
- The `NotificationSource` follows the same pattern as `MediaSource` in `sources/media.rs`
- Use a dedicated broadcast channel for notifications (separate from media broadcast)
- The existing IPC server broadcast plumbing already supports multiple sources
- Notification listening is signal-based (event-driven), not polling — no interval needed
- Use `zbus::MessageStream` or connection's `signal_stream()` for async signal handling
- No new Rust crates needed — zbus, serde, tokio already available
- Follow the same initialization pattern as `init_media_source()` / `start_media_poller()`
- For `NotificationAction`: GNOME Shell may not support `InvokeAction` directly; log as fallback
- For `NotificationClose`: call `org.freedesktop.Notifications.CloseNotification(u32)` D-Bus method
- The urgency value from the hints dict is a byte (u8); map to "low"/"normal"/"critical"

### Project Structure Notes

- New file: `dyd-daemon/src/sources/notifications.rs`
- New file: `dyd-ui/src/stores/notification-store.ts`
- Modify: `dyd-daemon/src/sources/mod.rs` (add `pub mod notifications;`)
- Modify: `dyd-daemon/src/lib.rs` (add NotificationSource field, init, start)
- Modify: `dyd-daemon/src/ipc/server.rs` (pass notification broadcast + source)
- Modify: `dyd-daemon/src/ipc/connection.rs` (real NotificationAction/Close handlers, notification source param)
- Modify: `dyd-ui/src/ipc/client.ts` (add NotificationReceived handler)
- Potentially: `types/src/lib.rs` and `types/src/index.ts` (extend NotificationPayload if needed)

### References

- [Freedesktop Notification Spec](https://specifications.freedesktop.org/notification-spec/latest/)
- [Source: existing `dyd-daemon/src/sources/media.rs` for source module pattern]
- [Source: existing `dyd-daemon/src/lib.rs` for Daemon struct and source initialization]
- [Source: existing `dyd-ui/src/stores/media-store.ts` for Zustand store pattern]
- [Source: existing `dyd-ui/src/ipc/client.ts` for IPC event handling pattern]
- [Source: existing `types/src/lib.rs` for NotificationPayload, IpcEvent::NotificationReceived]
- [Source: existing `dyd-daemon/src/ipc/connection.rs` for command handling pattern]

## Change Log

- 2026-07-20: Initial story creation
- 2026-07-20: Implemented NotificationSource module, wired into Daemon, updated IPC handlers, created notification store, extended types

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

- `dyd-daemon/src/sources/notifications.rs` implements NotificationSource following MediaSource pattern
- Used `Connection::call_method` to register D-Bus AddMatch with eavesdrop=true (zbus MatchRule doesn't support eavesdrop flag)
- `MessageStream::from(&conn)` used to receive all messages, filtered by interface/member in handler
- Added `futures` crate dependency for StreamExt usage
- `NotificationActionItem` struct added to types for action pairs from Notify call
- Refactored broadcast forwarding in connection.rs to use `spawn_broadcast_forwarder` helper function

### Completion Notes List

- Created `NotificationSource` in `sources/notifications.rs` with D-Bus message interception via AddMatch with eavesdrop, MessageStream-based listening, and extracted Notify params (app_name, summary, body, urgency, actions)
- Wired NotificationSource into Daemon struct: field, init/start methods, accessor, broadcast forwarding
- Replaced NotificationAction/NotificationClose stubs in connection.rs with real D-Bus call implementations
- Added NotificationSource reference passing through IPC server to connection handler
- Added `spawn_broadcast_forwarder` helper for generic broadcast fan-out to connection event channel
- Created notification-store.ts with setNotification/clearNotification Zustand store
- Added NotificationReceived case to UI client.ts with dispatch to notification store
- Extended NotificationPayload with actions field in both Rust and TypeScript types
- Added NotificationActionItem struct to Rust/TS types (key, label pairs from Notify actions array)
- All 12 existing tests pass, clean compilation on both Rust and TypeScript

### File List

- New: `dyd-daemon/src/sources/notifications.rs`
- New: `dyd-ui/src/stores/notification-store.ts`
- Modified: `dyd-daemon/src/sources/mod.rs` — added `pub mod notifications;`
- Modified: `dyd-daemon/src/lib.rs` — added NotificationSource field, init/start methods, accessor, wove into run()
- Modified: `dyd-daemon/src/ipc/server.rs` — added notification_broadcast + notification_source params
- Modified: `dyd-daemon/src/ipc/connection.rs` — real NotificationAction/Close handlers, notification_source param, spawn_broadcast_forwarder helper
- Modified: `dyd-daemon/Cargo.toml` — added `futures = "0.3"` dependency
- Modified: `dyd-ui/src/ipc/client.ts` — added NotificationReceived case
- Modified: `types/src/lib.rs` — added NotificationActionItem struct, actions field to NotificationPayload
- Modified: `types/src/index.ts` — added NotificationActionItem interface, actions field to NotificationPayload
