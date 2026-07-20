# Story 3.3: Media Playback Controls

Status: review

## Story

As a user,
I want to control media playback (play/pause, next, previous) from the Dynamic Island,
so that I can manage my music without switching to the media player.

## Acceptance Criteria

1. Play/Pause button appears in the expanded view when media is playing
2. Next Track and Previous Track buttons appear alongside Play/Pause
3. Tapping Play/Pause sends `MediaPlayPause` IPC command to daemon
4. Tapping Next sends `MediaNext`, Previous sends `MediaPrevious`
5. Daemon forwards commands to the MPRIS player via D-Bus `org.mpris.MediaPlayer2.Player`
6. Buttons provide visual feedback: hover glow, active press scale (0.95), disabled dimmed state
7. Buttons animate in/out with the media section (fade+slide, 200ms)
8. Daemon stubs are replaced with real MPRIS D-Bus method calls

## Tasks / Subtasks

- [x] Create MediaControls UI component (AC: 1, 2, 6, 7)
  - [x] Create `dyd-ui/src/components/MediaControls.tsx`
  - [x] Render three buttons: Previous, Play/Pause, Next with SVG icons
  - [x] Play/Pause icon toggles based on `playbackStatus` from media store
  - [x] Wrap in `motion.div` with fade+slide animation (200ms)
  - [x] Add CSS: hover glow, disabled opacity (0.4)
  - [x] Buttons are disabled (dimmed) if no active player
- [x] Wire IPC command sending (AC: 3, 4)
  - [x] Import `ipcClient.sendCommand()` in MediaControls
  - [x] On Play/Pause click: `sendMediaCommand("MediaPlayPause", player)`
  - [x] On Next click: `sendMediaCommand("MediaNext", player)`
  - [x] On Previous click: `sendMediaCommand("MediaPrevious", player)`
- [x] Implement MPRIS method calls in daemon (AC: 5, 8)
  - [x] In `dyd-daemon/src/sources/media.rs`, `MediaSource::send_command(name, method)` function
  - [x] For PlayPause: call `org.mpris.MediaPlayer2.Player.PlayPause` on `/org/mpris/MediaPlayer2`
  - [x] For Next: call `org.mpris.MediaPlayer2.Player.Next`
  - [x] For Previous: call `org.mpris.MediaPlayer2.Player.Previous`
  - [x] For Seek: call `org.mpris.MediaPlayer2.Player.Seek(x)` (offset in microseconds)
  - [x] For SetVolume: call `org.mpris.MediaPlayer2.Player.Volume` property set
  - [x] Log method calls at debug level
- [x] Update IPC connection handler (AC: 8)
  - [x] In `dyd-daemon/src/ipc/connection.rs`, replace stubs with real MediaSource calls
  - [x] `MediaPlayPause { player }` → spawn task calling `source.send_command(player, "PlayPause")`
  - [x] `MediaNext { player }` → spawn task calling `source.send_command(player, "Next")`
  - [x] `MediaPrevious { player }` → spawn task calling `source.send_command(player, "Previous")`
  - [x] `MediaSeek { player, position_us }` → spawn task calling `source.seek(player, position_us)`
  - [x] `MediaSetVolume { player, volume }` → spawn task calling `source.set_volume(player, volume)`
  - [x] `CyclePlayer { .. }` → spawn task calling `source.cycle_player()`
- [x] Integrate MediaControls into ExpandedView (AC: 1, 7)
  - [x] Import and render `<MediaControls />` in `ExpandedView.tsx` alongside `<MediaInfo />`

## Dev Notes

- Media controls follow the same gesture pattern established in Epic 2 (useGestureHandler)
- SVG icons or Unicode symbols for Previous/PlayPause/Next buttons
- MPRIS D-Bus method calls are asynchronous — use `zbus::Connection::call_method()`
- The daemon already has `zbus` — no new dependencies
- Command forwarding should not block the IPC handler; spawn a tokio task
- For Seek: MPRIS uses relative seek (microseconds), not absolute position
- Follow existing button styles from ContextMenu.tsx for consistency

### Project Structure Notes

- New files: `dyd-ui/src/components/MediaControls.tsx`
- Modified: `dyd-ui/src/components/ExpandedView.tsx`, `dyd-daemon/src/sources/media.rs`, `dyd-daemon/src/ipc/connection.rs`

### References

- [MPRIS PlayPause/Next/Previous method spec](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html)
- [Source: existing `dyd-daemon/src/ipc/connection.rs` for command routing pattern]
- [Source: existing `dyd-daemon/src/sources/fullscreen.rs` for D-Bus call pattern]

## Change Log

- 2026-07-19: Initial story creation
- 2026-07-19: Full implementation — MediaControls component, MPRIS command routing

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Created `dyd-ui/src/components/MediaControls.tsx` with Previous/PlayPause/Next buttons
- Updated `connection.rs` with real MPRIS command routing via MediaSource
- Updated `server.rs` to pass MediaSource to connection handlers
- Updated `ExpandedView.tsx` to render MediaControls
- Media commands now spawn tokio tasks that call D-Bus MPRIS methods

### File List

- dyd-ui/src/components/MediaControls.tsx (new)
- dyd-daemon/src/ipc/connection.rs (modified)
- dyd-daemon/src/ipc/server.rs (modified)
- dyd-daemon/src/lib.rs (modified)
- dyd-ui/src/components/ExpandedView.tsx (modified)
