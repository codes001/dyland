# Story 3.5: Multi-Player Support

Status: review

## Story

As a user,
I want the Dynamic Island to handle multiple active media players gracefully,
so that the most relevant player is always shown without conflict.

## Acceptance Criteria

1. Daemon tracks all active MPRIS players (e.g., Spotify + VLC + Firefox simultaneously)
2. Active player = the one that last changed PlaybackStatus to "Playing"
3. If multiple players are Playing, prioritize: most recent state change wins
4. If all players are Paused/Stopped, show the most recently paused player
5. When active player disappears, fall back to next available playing player
6. UI shows player name badge (e.g., "Spotify") alongside the media info
7. User can long-press the media area to switch active player (cycle through available players)
8. Player switching sends updated MediaChanged events to UI

## Tasks / Subtasks

- [x] Implement multi-player tracking in daemon (AC: 1, 2, 3, 4, 5)
  - [x] In `sources/media.rs`, maintain `HashMap<String, PlayerState>` of tracked players
  - [x] `PlayerState`: player_name, metadata, playback_status, position, last_active_timestamp
  - [x] Update `last_active_timestamp` when PlaybackStatus changes to Playing
  - [x] Implement `select_active_player()`: returns player with latest last_active where Playing, else latest where Paused
  - [x] On player disappearance, remove from map and re-select active player
  - [x] If selected player changes, emit new MediaChanged event
- [x] Add player name to UI (AC: 6)
  - [x] In `MediaInfo.tsx`, render player name as a small badge/tag above the title
  - [x] Badge style: `text-[10px] bg-white/5 rounded-full px-2 py-0.5`
- [x] Implement player cycling on long-press (AC: 7)
  - [x] Register long-press gesture handler (500ms) on MediaInfo component
  - [x] On long-press, send `CyclePlayer` IPC command to daemon
  - [x] `CyclePlayer` IPC command defined in types (both Rust and TS)
  - [x] Daemon rotates active player to next in tracked list via `cycle_player()`
  - [x] Emit updated MediaChanged event for new active player via broadcast
- [x] Handle edge cases (AC: 4, 5)
  - [x] All players disappear → clear media, return to dormant state
  - [x] New player appears while another is active → do NOT auto-switch unless new player starts Playing
  - [x] Player becomes Playing while another is also Playing → new player wins (last_active_timestamp)
  - [x] Single player pause/resume → normal behavior, no player switch

## Dev Notes

- Multi-player is a key differentiator — Apple Dynamic Island only handles one media source
- The player tracking map lives in the daemon, synced to UI via MediaChanged events
- `last_active_timestamp` is `tokio::time::Instant` for ordering comparisons
- The `CyclePlayer` command needs to be added to both Rust and TS types
- Use the existing IPC command/event extension pattern
- Long-press is already handled via `useGestureHandler` from Epic 2 — reuse it
- Player badge helps users identify which app is producing audio
- Edge case: Firefox may register multiple MPRIS instances per tab — handle by grouping or showing only the active one

### Project Structure Notes

- Modified: `dyd-daemon/src/sources/media.rs`, `types/src/lib.rs`, `types/src/index.ts`, `dyd-ui/src/components/MediaInfo.tsx`, `dyd-daemon/src/ipc/connection.rs`

### References

- [MPRIS name ownership spec](https://specifications.freedesktop.org/mpris-spec/latest/#bus-name-policy)
- [Source: existing `dyd-ui/src/hooks/useGestureHandler.ts` for long-press detection]
- [Source: existing `types/src/lib.rs` and `types/src/index.ts` for IPC type definitions]

## Change Log

- 2026-07-19: Initial story creation
- 2026-07-19: Full implementation — multi-player tracking, player badge, long-press cycling

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Multi-player tracking implemented in media.rs: HashMap<String, PlayerState> with last_active timestamps
- `select_active_player()` prioritizes Playing > Paused by last_active_timestamp
- `cycle_player()` rotates through tracked players and emits MediaChanged
- Player badge rendered in MediaInfo.tsx with pill-style label
- Long-press on media area sends CyclePlayer IPC command (500ms hold)
- `CyclePlayer` command added to types/src (both Rust and TS)

### File List

- dyd-daemon/src/sources/media.rs (modified)
- dyd-ui/src/components/MediaInfo.tsx (modified)
- types/src/lib.rs (modified)
- types/src/index.ts (modified)
