# Story 5.4: Volume Control

Status: review

## Story

As a system,
I want to monitor and control system volume via PipeWire/WirePlumber (fallback PulseAudio),
so that users can see current volume level in the pill and adjust it with a slider in the expanded view.

## Acceptance Criteria

1. Daemon monitors the default audio sink for volume (0.0-1.0), mute state, and sink name
2. Changes emitted to UI via IPC (`VolumeChanged` event) within 200ms
3. No audio hardware → volume indicator hidden
4. Pill shows `[volume icon] + [percentage or level indicator]`; icon reflects mute (X), low (<33% 1 wave), medium (33-66% 2 waves), high (>66% 3 waves)
5. Expanded view shows volume slider (0-100%, draggable), mute/unmute toggle, active output device name
6. Slider changes volume in real-time via `SetVolume` command
7. Mute toggle sends `SetMute` command
8. Volume at 0 → always show muted icon regardless of mute state

## Tasks / Subtasks

- [x] Add `VolumeChanged` event + `VolumePayload` to types (AC: 2, 4)
  - [x] Add `VolumeChanged` variant to `IpcEvent` in `types/src/lib.rs`
  - [x] Add `VolumePayload` struct: volume (f64), muted (bool), sink_name (Option<String>)
  - [x] Mirror in `types/src/index.ts`
- [x] Implement VolumeSource module in daemon (AC: 1, 2, 3, 6, 7)
  - [x] Create `dyd-daemon/src/sources/volume.rs` with `VolumeSource` struct
  - [x] Detect available tool: prefer `wpctl` (WirePlumber/PipeWire), fallback to `pactl` (PulseAudio/PipeWire-pulse)
  - [x] Read default sink volume + mute via subprocess (`wpctl get-volume` / `pactl get-sink-volume` + `pactl get-sink-mute`)
  - [x] Poll every 200ms to detect changes
  - [x] Parse volume fraction (0.0-1.0) and mute state; clamp display to 100%
  - [x] No audio tool or no sink → volume hidden
  - [x] Build `VolumePayload` and emit `IpcEvent::VolumeChanged` via broadcast
  - [x] Implement `set_volume(volume)` → `wpctl set-volume` / `pactl set-sink-volume`
  - [x] Implement `set_mute(muted)` → `wpctl set-mute toggle` / `pactl set-sink-mute toggle`
  - [x] Handle errors gracefully (subprocess failures logged, no crash)
- [x] Wire VolumeSource into Daemon struct (AC: 1, 2)
  - [x] Add `volume_source: Option<Arc<VolumeSource>>` to Daemon struct
  - [x] Add `init_volume_source()` and `start_volume_poller()`
  - [x] Call init + start in `Daemon::run()`
  - [x] Pass volume broadcast + source to IPC server startup
- [x] Update IPC server/connection wiring (AC: 6, 7)
  - [x] Add `volume_broadcast` param to `ipc/server.rs::start()`
  - [x] Add `volume_broadcast_rx` param + forwarder in `ipc/connection.rs`
  - [x] Replace `SetVolume`/`SetMute` stubs in `connection.rs:277` with real source calls
- [x] Handle VolumeChanged in UI IPC client (AC: 2)
  - [x] Add `case "VolumeChanged"` to switch in `client.ts` dispatching to system store
- [x] Extend system store with volume (AC: 2, 4)
  - [x] Add `volume` state and `setVolume(payload)` action to `dyd-ui/src/stores/system-store.ts`
- [x] Create VolumeIndicator component (AC: 4, 8)
  - [x] Create `dyd-ui/src/components/status/VolumeIndicator.tsx`
  - [x] Speaker icon with waves by level; X when muted or volume 0
- [x] Create VolumeExpandedView component (AC: 5, 6, 7)
  - [x] Create `dyd-ui/src/components/status/VolumeExpandedView.tsx`
  - [x] Range slider (0-100%) sending `SetVolume`; mute toggle sending `SetMute`; sink name display

## Dev Notes

- Prefer PipeWire `wpctl`; fall back to PulseAudio `pactl` (PipeWire-pulse ships `pactl` compat)
- `wpctl get-volume @DEFAULT_AUDIO_SINK@` output: `Volume: 0.85` or `Volume: 0.85 [MUTED]`
- `wpctl set-volume @DEFAULT_AUDIO_SINK@ <fraction>` (e.g., `0.5`)
- `wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle`
- `pactl get-sink-volume @DEFAULT_SINK@` → `Volume: front-left: 65536 / 100% / 0.00 dB`
- `pactl get-sink-mute @DEFAULT_SINK@` → `Mute: no` / `Mute: yes`
- `pactl set-sink-volume @DEFAULT_SINK@ 50%`
- `pactl set-sink-mute @DEFAULT_SINK@ toggle`
- Use `tokio::process::Command` for async subprocess calls
- Poll every 200ms to satisfy the 200ms emission AC (subprocess calls are fast)
- Clamp volume to 0.0-1.0 in `SetVolume` handler (existing clamp in connection.rs:214 pattern)
- Sink name: `wpctl get-volume` doesn't expose name; leave `sink_name` as None for wpctl, or parse `pactl list sinks short` default sink
- No new Rust crates needed — `tokio::process` requires tokio "process" feature (tokio "full" already enables it)
- Volume at 0 → UI always shows muted icon

### Project Structure Notes

- New file: `dyd-daemon/src/sources/volume.rs`
- New file: `dyd-ui/src/components/status/VolumeIndicator.tsx`
- New file: `dyd-ui/src/components/status/VolumeExpandedView.tsx`
- Modify: `dyd-daemon/src/sources/mod.rs` (add `pub mod volume;`)
- Modify: `dyd-daemon/src/lib.rs` (add VolumeSource field, init, start)
- Modify: `dyd-daemon/src/ipc/server.rs` (add volume_broadcast + volume_source params)
- Modify: `dyd-daemon/src/ipc/connection.rs` (add volume_broadcast_rx param + forwarder, real SetVolume/SetMute)
- Modify: `dyd-ui/src/ipc/client.ts` (add VolumeChanged case)
- Modify: `dyd-ui/src/stores/system-store.ts` (add volume state)
- Modify: `types/src/lib.rs` and `types/src/index.ts` (VolumeChanged + VolumePayload)

### References

- [WirePlumber wpctl man page](https://pipewire.pages.freedesktop.org/wireplumber/daemon/command-line/)
- [Source: existing `dyd-daemon/src/sources/media.rs` for polling pattern]
- [Source: existing `types/src/lib.rs` for IpcEvent variants]
- [Source: existing `dyd-daemon/src/ipc/connection.rs` for command handler pattern]

## Change Log

- 2026-08-02: Initial story creation
- 2026-08-02: Implemented VolumeSource (wpctl/pactl subprocess poll, set_volume/set_mute), wired into Daemon/SourceContext/IPC, real SetVolume/SetMute handlers, VolumeIndicator + VolumeExpandedView UI.

### Review Findings

- [x] [Review][Decision] Pill never shows volume percentage — 5-4 AC4 (icon + level) conflicts with 5-6 AC1 (compact 16px icon-only row). **Decision: show text in pill** — 5-6 AC1 amended; StatusIconRow reworked to show volume %. [dyd-ui/src/components/status/StatusIconRow.tsx]
- [x] [Review][Decision] SetMute ignores `muted` param and always toggles — spec Dev Notes mandated toggle, but IPC/UI contract sends an absolute target (`{muted: !muted}`); rapid double-click nets to no change and UI state can diverge. **Decision: leave as-is** — spec-compliant toggle, documented; not a defect against the story. [dyd-daemon/src/sources/volume.rs, connection.rs]
- [x] [Review][Patch] Tool detection caches by binary presence (`wpctl --version`), not by server health — if PipeWire is dead, wpctl is cached forever. Verify a `get-volume` succeeds before caching. [dyd-daemon/src/sources/volume.rs]
- [x] [Review][Patch] set_volume/set_mute ignore subprocess exit status — a failed `wpctl set-volume` still reports success to the client. [dyd-daemon/src/sources/volume.rs]
- [x] [Review][Patch] Slider drag fires unbounded subprocess calls (no debounce) — every onChange pixel spawns a `wpctl set-volume`. Debounce in VolumeExpandedView. [dyd-ui/src/components/status/VolumeExpandedView.tsx]
- [x] [Review][Defer] sink_name always None on wpctl path — sanctioned by 5-4 Dev Notes [dyd-daemon/src/sources/volume.rs]
- [x] [Review][Defer] 200ms poll subprocess churn — spec-mandated polling [dyd-daemon/src/sources/volume.rs]
- [x] [Review][Defer] NaN/Inf coerced to 0 in SetVolume/SetMute clamp — matches existing media-source clamp pattern [dyd-daemon/src/ipc/connection.rs]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Tool detection cached in RwLock (AudioTool::Wpctl/Pactl/None), re-verified only when None.
- pactl percent parse fixed: pactl output is `Volume: front-left: 32768 / 50% / -18.06 dB` — parsed via ` / ` then `%` boundary, not `/ 100%`.
- pactl sink name read via `pactl get-default-sink` (the Volume: line does not carry the sink name). wpctl leaves sink_name None per story notes.
- mute toggles use `toggle` (state is derived from poll each 200ms).
- 28 lib tests pass; typecheck clean.

### File List

- dyd-daemon/src/sources/volume.rs (new)
- dyd-daemon/src/sources/mod.rs
- dyd-daemon/src/lib.rs
- dyd-daemon/src/ipc/context.rs
- dyd-daemon/src/ipc/connection.rs
- dyd-ui/src/components/status/VolumeIndicator.tsx (new)
- dyd-ui/src/components/status/VolumeExpandedView.tsx (new)
- types/src/lib.rs, types/src/index.ts (already present)
