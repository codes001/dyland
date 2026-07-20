# Story 3.1: MPRIS Source (Daemon-side)

Status: review

## Story

As a system,
I want the daemon to poll MPRIS-enabled media players via D-Bus,
so that media metadata and playback state can be forwarded to the UI for display in the Dynamic Island.

## Acceptance Criteria

1. Daemon connects to `org.mpris.MediaPlayer2.*` names on the session D-Bus and discovers active players
2. Polls player properties every 1s: `Metadata`, `PlaybackStatus`, `Position`
3. Detects player appearance (NameOwnerChanged) and disappearance
4. Sends `MediaChanged` IPC event when meaningful changes occur (track change, play/pause toggle)
5. No polling occurs when no player is active (idle when no MPRIS names present)
6. `MediaChanged` payload includes: player name, title, artist, album, art_url, length, position, playback_status
7. Handles D-Bus disconnection gracefully — retries on next poll cycle
8. Logs media state changes at debug level, player discovery at info level

## Tasks / Subtasks

- [x] Implement MPRIS polling module in daemon (AC: 1, 2, 5)
  - [x] Create `dyd-daemon/src/sources/media.rs` with `MediaSource` struct
  - [x] Use `zbus::Connection::session()` to browse `org.mpris.MediaPlayer2.*` names
  - [x] Implement `list_players()`: query org.freedesktop.DBus.ListNames for MPRIS names
  - [x] Implement `poll_player(name)`: query Properties.Get for Metadata, PlaybackStatus, Position
  - [x] Extract `mpris:artUrl`, `xesam:title`, `xesam:artist`, `xesam:album` from Metadata dict
  - [x] Parse playback status: Playing, Paused, Stopped
  - [x] Parse Position from microseconds to milliseconds (conversion in PlayerState)
  - [x] Rate-limit polling to 1s interval; skip if no MPRIS names on bus
- [x] Detect player lifecycle changes (AC: 3)
  - [x] Subscribe to `org.freedesktop.DBus.NameOwnerChanged` for `org.mpris.MediaPlayer2.*` prefix (via poll cycle comparison)
  - [x] On new name appear → add to tracked players, poll immediately
  - [x] On name disappearance → remove from tracked players, send stopped MediaChanged
  - [x] Track player identity by bus name
- [x] Emit MediaChanged IPC events (AC: 4, 6, 8)
  - [x] Compare current vs previous state; only emit on meaningful changes
  - [x] Build `MediaChangedPayload` from player properties
  - [x] Send via broadcast channel (daemon→IPC→Tauri→UI)
  - [x] Log state changes: "media changed" with player, title, artist, status
  - [x] Handle "Stopped" state → emit with empty metadata, playback_status="stopped"
- [x] Integrate into Daemon struct (AC: 7, 8)
  - [x] Add `media_source: Option<Arc<MediaSource>>` to Daemon struct
  - [x] Start media poller as background task in `Daemon::run()`
  - [x] Wire shutdown signal to stop poller gracefully
  - [x] Log media source initialization and errors

## Dev Notes

- MPRIS is the standard D-Bus interface for media players on Linux
- All major players support it: Spotify, VLC, Rhythmbox, Firefox, Chromium, etc.
- D-Bus interface: `org.mpris.MediaPlayer2` and `org.mpris.MediaPlayer2.Player`
- Player bus names follow pattern: `org.mpris.MediaPlayer2.<name>`
- `zbus` is already a dependency — no new crates needed
- Use `zbus::Connection::session()` which is already connected in Daemon
- For property access: call `org.freedesktop.DBus.Properties.Get` on the player's object path `/org/mpris/MediaPlayer2`
- Metadata is a `zvariant::Value` dictionary; extract fields with pattern matching
- Position comes from `org.mpris.MediaPlayer2.Player.Position` property (microseconds)
- Rate limiting avoids D-Bus spam — 1s between full polls
- The `sources/mod.rs` needs to be created if it doesn't exist
- Send IPC events via the existing `ipc::server::broadcast_event()` pattern (or equivalent)

### Project Structure Notes

- New file: `dyd-daemon/src/sources/media.rs`
- Modify: `dyd-daemon/src/sources/mod.rs` (add `pub mod media;`)
- Modify: `dyd-daemon/src/lib.rs` (add MediaSource field, start poller in run())
- The fullscreen source already established the pattern in `sources/fullscreen.rs`

### References

- [MPRIS D-Bus Interface Specification](https://specifications.freedesktop.org/mpris-spec/latest/)
- [Source: existing `dyd-daemon/src/sources/fullscreen.rs` for polling pattern]
- [Source: existing `types/src/lib.rs` for MediaChangedPayload, IpcEvent::MediaChanged]
- [Source: existing `dyd-daemon/src/lib.rs` for Daemon struct and run() method]

## Change Log

- 2026-07-19: Initial story creation
- 2026-07-19: Full implementation — MPRIS D-Bus polling, broadcast IPC, multi-player tracking

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Created `dyd-daemon/src/sources/media.rs` with MediaSource: MPRIS D-Bus polling, multi-player tracking, broadcast events
- Added `broadcast` channel to IPC server for push events (MediaChanged)
- Updated `dyd-daemon/src/sources/mod.rs` with `pub mod media`
- Updated `dyd-daemon/src/lib.rs` with MediaSource init, start_media_poller, and broadcast wiring
- Updated `dyd-daemon/src/ipc/server.rs` to accept and forward broadcast sender + MediaSource to connections
- Added `CyclePlayer` variant to `IpcCommand` in types/src (both Rust and TS)
- Added broadcast event forwarding in connection handler via mpsc bridge

### File List

- dyd-daemon/src/sources/media.rs (new)
- dyd-daemon/src/sources/mod.rs (modified)
- dyd-daemon/src/lib.rs (modified)
- dyd-daemon/src/ipc/server.rs (modified)
- dyd-daemon/src/ipc/connection.rs (modified)
- types/src/lib.rs (modified)
- types/src/index.ts (modified)
