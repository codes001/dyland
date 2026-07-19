# Story 2.1: State Machine

Status: ready-for-dev

## Story

As a system,
I want a deterministic state machine driving the Dynamic Island through Dormant, Active, Expanded, and Hidden states,
so that the pill has a predictable, testable lifecycle that all visual rendering and interactions map to.

## Acceptance Criteria

1. Four states defined: `dormant` (126×36px, no content, input passthrough), `active` (width grows, icon+text, interactive), `expanded` (380×200+px, full content, fully interactive), `hidden` (off-screen, fullscreen)
2. State transitions are deterministic with documented transition table (source, target, duration, easing, content animation)
3. Transitions are interruptible — new event mid-animation cancels current animation cleanly
4. Active state auto-dismisses to dormant after configurable timeout (default 4s) unless user is interacting
5. TypeScript enum `'dormant' | 'active' | 'expanded' | 'hidden'` used throughout
6. All states can transition to hidden (fullscreen), hidden returns to previous state

## Tasks / Subtasks

- [ ] Implement state definitions and TypeScript enum (AC: 1, 5)
  - [ ] Create `PillState` type as `'dormant' | 'active' | 'expanded' | 'hidden'`
  - [ ] Define state machine rules: dormant↔active, active↔expanded, *↔hidden
  - [ ] Store state in Zustand store with actions for each transition
- [ ] Implement deterministic transitions with animation config (AC: 2)
  - [ ] Create transition table matching: dormant→active (300ms ease-out), active→dormant (200ms ease-in), active→expanded (400ms spring), expanded→active (300ms spring), →hidden (100ms ease-out), hidden→prev (150ms ease-out)
  - [ ] Wire transitions to Zustand store actions
  - [ ] Ensure each transition has defined content animation direction
- [ ] Implement interruptible animations (AC: 3)
  - [ ] New event during animation cancels current, starts new from current visual state
  - [ ] No visual glitch or flash during interruption
  - [ ] Use Motion's `AnimatePresence` with `mode="popLayout"`
  - [ ] Preserve spring velocity on interrupt
- [ ] Implement idle timeout and auto-dismiss (AC: 4)
  - [ ] Active state auto-dismisses to dormant after 4s configurable timeout
  - [ ] Timer resets on user interaction (hover, click) and new event arrival
  - [ ] No timeout when in expanded state
  - [ ] Use `useEffect` with `setTimeout`/`clearTimeout`, store timer ID in ref
- [ ] Implement hidden state for fullscreen (AC: 6)
  - [ ] `*` → `hidden` when fullscreen detected
  - [ ] `hidden` → previous state on fullscreen exit
  - [ ] Track previous state to restore correctly

## Dev Notes

- Use Zustand store for state management (Pattern: `src/stores/`). Already established in Epic 1.
- UI Component tree: `<DynamicIsland>` → `<Pill>` with `<Gestures>` wrapper
- State drives all rendering — components read state from Zustand selectors
- Motion library already imported via Epic 1 scaffolding (check `package.json` for `framer-motion` or `motion`)
- Refer to SYSTEM-DESIGN.md §State Machine for the authoritative transition diagram
- Refer to EPIC-02-PILL-RENDERING.md §Feature 2.1 for full user story details
- Source hints: [Source: bmad-output/EPIC-02-PILL-RENDERING.md#Feature-21-State-Machine], [Source: bmad-output/SYSTEM-DESIGN.md#State-Machine]

### Project Structure Notes

- Story file: `_bmad-output/implementation-artifacts/2-1-state-machine.md`
- Target code: `dyd-ui/src/stores/pill-store.ts` (new), `dyd-ui/src/types/pill.ts` (new)
- Follow Epic 1 patterns for store structure (Zustand with TypeScript)

### References

- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#Feature-21-State-Machine]
- [Source: bmad-output/SYSTEM-DESIGN.md#State-Machine]
- [Source: bmad-output/SYSTEM-DESIGN.md#Component-Tree-React]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List
