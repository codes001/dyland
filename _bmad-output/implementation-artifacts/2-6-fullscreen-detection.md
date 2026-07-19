# Story 2.6: Fullscreen Detection

Status: ready-for-dev

## Story

As a system,
I want to detect when a fullscreen application is active on the same monitor as the pill,
so that the Dynamic Island hides itself (hidden state) and restores when fullscreen ends.

## Acceptance Criteria

1. Fullscreen window detected within 500ms via D-Bus (`org.gnome.Shell.Introspect`) or wlr-foreign-toplevel-management protocol
2. Pill transitions to hidden state with smooth slide-up animation (100ms)
3. Pill reappears in previous state when fullscreen ends (slide-down 150ms)
4. Only hides when fullscreen window is on the same monitor
5. Activity queue items accumulated during hidden period are shown on restore (oldest first)
6. "Keep visible during fullscreen" toggle in settings overrides behavior

## Tasks / Subtasks

- [ ] Implement Wayland fullscreen detection in daemon (AC: 1, 4)
  - [ ] Primary: `org.gnome.Shell.Introspect.GetWindows()` via zbus for GNOME
  - [ ] Fallback: `zwlr_foreign_toplevel_management_v1` for wlroots compositors
  - [ ] Poll interval: 2 seconds
  - [ ] Cache fullscreen state to avoid repeated D-Bus calls
  - [ ] Track which monitor the fullscreen window is on
  - [ ] Only emit `FullscreenChanged` if fullscreen is on pill's monitor or unknown
- [ ] Implement UI side: hide/show pill on fullscreen (AC: 2, 3, 5)
  - [ ] Listen for `FullscreenChanged` IPC event
  - [ ] Transition pill to hidden state (slide-up 100ms)
  - [ ] Restore to previous state on fullscreen exit (slide-down 150ms)
  - [ ] Track previous state before hiding
  - [ ] On restore: show oldest activity from queue
- [ ] Implement fullscreen exit restoration (AC: 3, 5)
  - [ ] Track fullscreen window count per monitor
  - [ ] Count reaches 0 → trigger restore
  - [ ] Handle window destruction without explicit fullscreen-exit (poll active windows)
- [ ] Implement manual override setting (AC: 6)
  - [ ] "Keep visible during fullscreen" toggle in settings
  - [ ] When enabled, ignore fullscreen events (pill stays visible)
  - [ ] Setting persisted in daemon config
  - [ ] Toggle while in fullscreen → apply immediately
  - [ ] IPC command: `UpdateConfig { changes: { fullscreen_override: true } }`

## Dev Notes

- Daemon-side detection in `dyd-daemon/src/sources/fullscreen/` or integrated into existing sources
- Wayland fullscreen detection is technically challenging — multiple fallback strategies needed
- IPC event `FullscreenChanged` already defined in SYSTEM-DESIGN.md §IPC Protocol
- UI store should have `isFullscreen` state that drives pill to hidden state
- Refer to EPIC-02-PILL-RENDERING.md §Feature 2.6 for detailed specs
- Config schema in SYSTEM-DESIGN.md §Configuration Schema already has room for appearance settings

### Project Structure Notes

- Daemon: `dyd-daemon/src/sources/fullscreen/mod.rs` (new module)
- UI: zustand store update or direct IPC event handler in pill component
- Setting persistence follows Epic 1 config pattern

### References

- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#Feature-26-Fullscreen-Detection]
- [Source: bmad-output/SYSTEM-DESIGN.md#IPC-Protocol-Specification]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List
