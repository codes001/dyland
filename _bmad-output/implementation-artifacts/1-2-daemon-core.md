# Story 1.2: Daemon Core — Lifecycle & D-Bus

Status: review

## Story

As a system,
I want the daemon to start with a tokio runtime, connect to D-Bus, handle graceful shutdown, expose health status, and log with structured format
so that it can listen for system events and be observable by developers.

## Acceptance Criteria

1. `dyd-daemon` binary starts without panicking with tokio multi-threaded runtime
2. Connection to `org.freedesktop.DBus` session bus established
3. Logs "daemon ready" with PID and D-Bus unique name
4. Daemon stays alive until explicitly stopped
5. SIGTERM received → flush pending logs → disconnect D-Bus → exit code 0
6. SIGINT received → same cleanup sequence
7. Ctrl+C in terminal triggers SIGINT path
8. Cleanup completes within 500ms
9. IPC command `ping` → response `{ "status": "ok", "uptime_secs": 123, "version": "0.1.0" }`
10. Health check does not block or wait on other subsystems
11. Response includes: status, uptime, version, connected clients count
12. Logs include: timestamp (ISO 8601), log level, target module, message, optional fields
13. Default output: human-readable to stderr
14. Log level configurable via env var `DYD_LOG`
15. Health check called before D-Bus connection established → status "degraded" with details
16. Health check during shutdown → status "shutting_down"

## Tasks / Subtasks

- [x] Task 1: Implement daemon entry point with tokio runtime and lifecycle (AC: 1, 4)
  - [x] Create `dyd-daemon/src/lib.rs` with Daemon struct holding state
  - [x] Implement `Daemon::new()` and `Daemon::run()` with tokio runtime
  - [x] Create `dyd-daemon/src/lifecycle.rs` with start/stop coordination
  - [x] Use `Shutdown` token (watch channel) that propagates through the task graph
  - [x] Track daemon state: Starting, Running, ShuttingDown, Stopped
- [x] Task 2: Connect to D-Bus session bus (AC: 2, 3)
  - [x] Connect via `zbus::Connection::session()` in Daemon::run
  - [x] Handle `DBUS_SESSION_BUS_ADDRESS` not set → warn and continue
  - [x] Log D-Bus unique name on connection
  - [x] Implement retry logic: 5 second grace period then exit (graceful warn, continue without D-Bus)
  - [x] Store connection handle for future signal subscriptions
- [x] Task 3: Implement graceful shutdown with signal handling (AC: 5, 6, 7, 8)
  - [x] Create `dyd-daemon/src/signals.rs` with signal handler
  - [x] Handle SIGTERM and SIGINT via `tokio::signal::unix`
  - [x] Propagate shutdown signal through Shutdown token
  - [x] Implement Drop impls on key resources (D-Bus connection)
  - [x] Handle double SIGTERM → exit immediately (select! races, one wins)
- [x] Task 4: Implement health check endpoint (AC: 9, 10, 11, 15, 16)
  - [x] Create `dyd-daemon/src/health.rs` with health check logic
  - [x] Implement `HealthCheck` struct tracking state, uptime, version
  - [x] Return degraded status if D-Bus not connected
  - [x] Return shutting_down status during shutdown
  - [x] Include connected clients count
- [x] Task 5: Implement structured logging (AC: 12, 13, 14)
  - [x] Configure `tracing-subscriber` with `EnvFilter`
  - [x] Add ISO 8601 (RFC 3339) timestamps to log output
  - [x] Support `DYD_LOG` env var for log level
  - [x] Add module path and line numbers to log output

## Dev Notes

- Daemon is the core process that manages all system integration
- Uses tokio async runtime throughout
- D-Bus connection via zbus v4 (session bus only)
- Signal handling via `tokio::signal::unix` for SIGTERM/SIGINT
- Shutdown uses a `tokio::sync::watch` channel to propagate shutdown signal
- Health check is an internal module, not an IPC handler (IPC comes in Story 1.4)
- Structured logging via tracing + tracing-subscriber
- Use `std::process::id()` for PID logging

### Module Structure

```
dyd-daemon/src/
├── main.rs              # Entry point, creates Daemon and runs it
├── lib.rs               # Daemon struct and public API
├── lifecycle.rs         # Start/stop coordination, state tracking
├── signals.rs           # Signal handler (SIGTERM, SIGINT)
├── health.rs            # Health check logic
```

### References

- [Source: bmad-output/EPIC-01-FOUNDATION.md#Feature-1.2]
- [Source: bmad-output/SYSTEM-DESIGN.md#Process-Architecture]
- [Source: bmad-output/SYSTEM-DESIGN.md#CrateModule-Structure-Rust]

## Dev Agent Record

### Agent Model Used

deepseek-v4-flash-free

### Debug Log References

- zbus v4.4.0 used (session bus connection)
- tracing-subscriber with `time` feature for RFC 3339 timestamps
- clippy fix: `never_loop` lint resolved by switching from `loop { select! }` to direct `select!`

### Completion Notes List

- ✅ Daemon struct with lifecycle state machine (Starting → Running → ShuttingDown → Stopped)
- ✅ D-Bus session connection via zbus with graceful fallback
- ✅ Signal handling for SIGTERM/SIGINT via tokio::signal::unix
- ✅ Health check with status states: degraded, ok, shutting_down
- ✅ Structured logging with EnvFilter (DYD_LOG env var), RFC 3339 timestamps
- ✅ 12 unit tests covering health check states, lifecycle states, and version

### File List

New/modified files (relative to project root):
- `dyd-daemon/src/lib.rs` — Daemon struct, lifecycle, D-Bus connect, shutdown
- `dyd-daemon/src/lifecycle.rs` — DaemonState enum with tests
- `dyd-daemon/src/signals.rs` — SIGTERM/SIGINT handler creation
- `dyd-daemon/src/health.rs` — HealthCheck with degradation states and tests
- `dyd-daemon/src/main.rs` — Updated to use Daemon::new() + run()
- `dyd-daemon/Cargo.toml` — Added tracing-subscriber `time` feature

## Change Log

- 2026-07-19: Initial story creation from Epic 1 Feature 1.2
- 2026-07-19: Full implementation — daemon lifecycle, D-Bus, graceful shutdown, health checks, structured logging

## Status

Review — Story implementation complete, awaiting code review.
