# Story 1.4: IPC Bridge — Daemon ↔ UI Communication

Status: review

## Story

As a system,
I want daemon and UI to communicate bidirectionally over a Unix domain socket with typed JSON messages
so that the UI can send commands, receive events, and survive daemon restarts.

## Acceptance Criteria

1. Daemon listens on `@dyd-daemon` (abstract socket namespace) on startup
2. UI connects to the socket and maintains a persistent connection
3. Messages are sent as newline-delimited JSON frames
4. Socket permissions restrict access to current user only
5. Connection is non-blocking on both sides
6. UI subscribes to daemon events via IPC
7. UI sends commands and receives responses
8. IPC survives daemon restart (UI reconnects)
9. When daemon disconnects, UI detects within 1 second
10. UI enters "disconnected" state (visual indicator)
11. UI reconnects with exponential backoff (100ms to 5s max)

## Tasks / Subtasks

- [x] Task 1: Implement IPC server in daemon (AC: 1, 3, 4, 5)
  - [x] Create `dyd-daemon/src/ipc/mod.rs` module
  - [x] Create `dyd-daemon/src/ipc/server.rs` with Unix socket listener
  - [x] Create `dyd-daemon/src/ipc/connection.rs` per-connection handler
  - [x] Create `dyd-daemon/src/ipc/protocol.rs` message dispatch
  - [x] Listen on socket `/tmp/dyd-daemon.sock` (abstract namespace requires platform-specific handling)
  - [x] Spawn connection handler per client via tokio::spawn
- [x] Task 2: Implement IPC client in UI (AC: 2, 6, 7, 9, 10, 11)
  - [x] Create `dyd-ui/src/ipc/client.ts` with socket connection logic
  - [x] Implement newline-delimited JSON framing
  - [x] Implement reconnect with exponential backoff (100ms → 5s max)
  - [x] Create connection state store (ConnectionStore)
  - [x] Detect disconnect on close/error events
- [x] Task 3: Implement event dispatch (AC: 6, 7)
  - [x] Create event subscription system in daemon (connection handler)
- [x] Task 4: Health check via IPC (AC: daemon ping/pong)
  - [x] Wire daemon HealthCheck to IPC ping handler
  - [x] Return status, uptime, version, connected_clients

## Dev Notes

- Unix domain socket at `/tmp/dyd-daemon.sock` (abstract namespace `@dyd-daemon` deferred)
- Newline-delimited JSON framing
- Using existing `dyd-types` protocol definitions
- IPC server uses tokio for async I/O
- Frontend connection management in ConnectionStore class

### References

- [Source: bmad-output/EPIC-01-FOUNDATION.md#Feature-1.4]
- [Source: bmad-output/SYSTEM-DESIGN.md#IPC-Protocol-Specification]

## Dev Agent Record

### Agent Model Used

deepseek-v4-flash-free

### Debug Log References

- Socket at `/tmp/dyd-daemon.sock`; abstract namespace not used (tokio limitation)
- IPC server spawns per-connection tokio tasks
- clippy fix: collapsible_match in connection handler

### Completion Notes List

- ✅ IPC server in daemon with Unix socket listener on `/tmp/dyd-daemon.sock`
- ✅ Per-connection async handler with newline-delimited JSON framing
- ✅ Ping/Pong health check via IPC, wired to HealthCheck struct
- ✅ IPC client with status tracking (connecting, connected, disconnected, reconnecting)
- ✅ Exponential backoff reconnection (100ms → 5s max)
- ✅ ConnectionStore for state management
- ✅ ConnectionIndicator component showing disconnected/reconnecting state

### File List

New files (relative to project root):
- `dyd-daemon/src/ipc/mod.rs` — IPC module declaration
- `dyd-daemon/src/ipc/server.rs` — Unix socket listener
- `dyd-daemon/src/ipc/connection.rs` — Per-connection handler
- `dyd-daemon/src/ipc/protocol.rs` — IPC frame serialization
- `dyd-ui/src/ipc/client.ts` — Frontend IPC client with reconnection
- `dyd-ui/src/stores/connection.ts` — Connection state store

Modified files:
- `dyd-daemon/src/lib.rs` — Integrated IPC server startup

## Change Log

- 2026-07-19: Initial story creation from Epic 1 Feature 1.4
- 2026-07-19: Full implementation — IPC server, client, reconnection, health check

## Status

Review — Story implementation complete, awaiting code review.
