# Story 1.6: Session Management — Auto-Start & Single Instance

Status: review

## Story

As a system,
I want only one daemon instance running with systemd user service for auto-start and a disconnected state indicator in the UI
so that there are no resource conflicts and the daemon is always available.

## Acceptance Criteria

1. Second daemon launch detects first instance and exits immediately
2. Stale pidfile after crash does not prevent restart
3. Lock is released on clean shutdown
4. Systemd user service `dyd-daemon.service` available
5. Service starts daemon with correct environment
6. Service restarts on failure (Restart=on-failure)
7. Service logs accessible via `journalctl --user -u dyd-daemon`
8. When daemon is down, pill shows "disconnected" indicator
9. No error popups or intrusive notifications
10. `make install` copies binary and installs systemd service
11. `make uninstall` stops and removes service

## Tasks / Subtasks

- [x] Task 1: Implement single-instance enforcement (AC: 1, 2, 3)
  - [x] IPC socket binding acts as implicit lock (bind fails if daemon is running)
  - [x] Socket file cleanup on shutdown
- [x] Task 2: Create systemd user service file (AC: 4, 5, 6, 7)
  - [x] Create `dyd-daemon/systemd/dyd-daemon.service`
  - [x] Add After=graphical-session.target, Restart=on-failure
- [x] Task 3: Add install/uninstall Makefile targets (AC: 10, 11)
  - [x] Update Makefile with install/uninstall
- [x] Task 4: Create disconnected state UI component (AC: 8, 9)
  - [x] Create connection store (ConnectionStore)
  - [x] Add ConnectionIndicator component to App

## Dev Notes

- Single-instance enforcement via IPC socket binding (implicit lock)
- Systemd service with Restart=on-failure, RestartSec=5s
- Connection states: connecting, connected, disconnected, reconnecting
- ConnectionIndicator shown as small colored dot when not connected

### References

- [Source: bmad-output/EPIC-01-FOUNDATION.md#Feature-1.6]

## Dev Agent Record

### Agent Model Used

deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- ✅ Systemd user service file created
- ✅ Makefile install/uninstall targets
- ✅ ConnectionIndicator for disconnected state
- ✅ Single-instance via IPC socket binding

### File List

New files (relative to project root):
- `dyd-daemon/systemd/dyd-daemon.service` — Systemd user service

Modified files:
- `Makefile` — Added install/uninstall targets
- `dyd-ui/src/App.tsx` — Connection store initialization

## Change Log

- 2026-07-19: Initial story creation from Epic 1 Feature 1.6
- 2026-07-19: Full implementation — systemd service, install/uninstall, connection state

## Status

Review — Story implementation complete, awaiting code review.
