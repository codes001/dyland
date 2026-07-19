# Story 2.4: Multi-Activity Queue

Status: ready-for-dev

## Story

As a system,
I want an activity queue managed in the daemon that handles multiple concurrent activities with priority ordering,
so that the Dynamic Island displays one activity at a time, cycling through with proper priority preemption.

## Acceptance Criteria

1. Queue supports push, pop, peek, remove-by-ID operations with FIFO ordering within same priority level
2. Five priority levels: CRITICAL (calls) > HIGH (timers) > MEDIUM (media) > LOW (notifications) > INFO (system status)
3. Current activity displays for 4s, then next in queue with slide transition
4. High-priority activities preempt lower-priority current display
5. Queue capacity: max 20 items (oldest LOW removed)
6. Queue state synced to UI over IPC
7. User can cycle forward/backward through queue via gestures
8. Queue persists to disk at `$XDG_STATE_HOME/dyd/queue.json` and restores on daemon start

## Tasks / Subtasks

- [ ] Implement activity queue data structure in daemon (AC: 1, 2, 5)
  - [ ] Create `Activity` struct: `id` (Uuid), `source`, `priority`, `timestamp`, `content`, `ttl` (optional)
  - [ ] Create `Priority` enum: CRITICAL, HIGH, MEDIUM, LOW, INFO
  - [ ] Use Rust `BinaryHeap` with custom ordering by priority + timestamp
  - [ ] Implement push, pop, peek, remove_by_id methods
  - [ ] Max 20 items: oldest LOW removed when full
  - [ ] Deduplicate by activity ID
- [ ] Implement sequential display with IPC sync (AC: 3, 6)
  - [ ] Current activity displays for 4s (timer in daemon)
  - [ ] Timer resets on user interaction, new events
  - [ ] After timeout → next activity with slide transition
  - [ ] After last activity → return to dormant
  - [ ] Sync queue state to UI over IPC events: `ActivityQueued`, `ActivityDequeued`, `ActivityChanged`, `QueueEmptied`
  - [ ] Handle cycle_next/cycle_prev IPC commands from UI
- [ ] Implement activity prioritization and preemption (AC: 4)
  - [ ] New activity with higher priority → preempt current display
  - [ ] Preempted activity remains in queue (not dismissed)
  - [ ] Equal priority → queue behind current
  - [ ] Interruption animation: current slides out, new slides in (200ms)
  - [ ] Configurable preemption behavior in settings
- [ ] Implement queue persistence (AC: 8)
  - [ ] Write to `$XDG_STATE_HOME/dyd/queue.json` on every mutation (debounced 500ms)
  - [ ] Use `serde_json` with atomic write (write + sync_all + rename)
  - [ ] Read on daemon startup, filter expired items (past TTL)
  - [ ] Handle corrupt file: log warning, start empty
  - [ ] Create state directory if not exists

## Dev Notes

- Daemon code in `dyd-daemon/src/queue/` module
- IPC events already defined in SYSTEM-DESIGN.md §IPC Protocol Specification
- Refer to IPC messages: `ActivityQueued`, `ActivityDequeued`, `ActivityChanged`, `QueueEmptied` (daemon→UI)
- Refer to IPC commands: `DismissActivity`, `CycleNext`, `CyclePrev` (UI→daemon)
- Zustand store on UI side receives IPC events and updates activity queue state
- Refer to EPIC-02-PILL-RENDERING.md §Feature 2.4 for detailed queue specs
- The UI IPC client should already be wired from Epic 1 (1-4-ipc-bridge)

### Project Structure Notes

- Daemon queue module: `dyd-daemon/src/queue/mod.rs`, `activity.rs`, `priority.rs`
- UI Zustand store for activity: `dyd-ui/src/stores/activity-store.ts`
- Follow existing patterns from Epic 1's daemon structure

### References

- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#Feature-24-Multi-Activity-Queue]
- [Source: bmad-output/SYSTEM-DESIGN.md#IPC-Protocol-Specification]
- [Source: bmad-output/SYSTEM-DESIGN.md#CrateModule-Structure-Rust]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List
