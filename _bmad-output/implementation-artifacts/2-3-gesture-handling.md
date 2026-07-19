# Story 2.3: Gesture Handling

Status: ready-for-dev

## Story

As an end user,
I want to interact with the Dynamic Island using click (expand), swipe (dismiss), and long-press (context menu) gestures,
so that I can control the island naturally with both mouse and touch input.

## Acceptance Criteria

1. Click on active pill expands to full content view; click outside expanded view collapses it
2. Swipe down on active pill dismisses current activity (threshold: 30px distance or 200px/s velocity)
3. Long-press (500ms hold) shows context menu with activity-specific quick actions
4. Gesture conflict resolution: click, swipe, and long-press do not interfere with each other
5. Gestures work identically on mouse and touch input
6. Dormant state does not respond to gestures (input passthrough)

## Tasks / Subtasks

- [ ] Implement click to expand (AC: 1, 6)
  - [ ] Click on dormant pill → no action (input passthrough)
  - [ ] Click on active pill → transition to expanded state
  - [ ] Click outside expanded island → return to active state
  - [ ] Click on interactive content inside expanded → no collapse (use `stopPropagation`)
  - [ ] Handle double-click gracefully
- [ ] Implement swipe to dismiss (AC: 2, 5)
  - [ ] Track pointer events: `onPointerDown`, `onPointerMove`, `onPointerUp`
  - [ ] Swipe down threshold: 30px distance OR 200px/s velocity
  - [ ] During swipe: pill follows finger with rubber-banding (offset = threshold + (distance - threshold) * 0.3)
  - [ ] Below threshold: snap back to active (animated)
  - [ ] Above threshold: dismiss activity, return to dormant (animated)
  - [ ] Swipe up: no action (or collapse expanded→active)
  - [ ] Lock swipe to vertical direction only
  - [ ] Same logic for mouse and touch (pointer events handle both)
- [ ] Implement long-press context menu (AC: 3)
  - [ ] `onPointerDown` + `setTimeout(500ms)` to detect hold
  - [ ] Visual feedback: scale up to 1.05 at 300ms (haptic-like pulse)
  - [ ] Cancel timeout on `onPointerUp`, `onPointerMove` (>10px), or `onPointerCancel`
  - [ ] Context menu popup below pill (floating UI component)
  - [ ] Menu options depend on current activity (media: Skip/Play/Pause, notification: Dismiss, etc.)
  - [ ] Only one context menu at a time
- [ ] Implement gesture conflict resolution (AC: 4)
  - [ ] Gesture recognizer state machine: PENDING→SWIPE (move>10px), PENDING→LONG_PRESS (500ms), PENDING→CLICK (pointerUp<300ms)
  - [ ] First recognizer to match cancels the other two
  - [ ] Single `onPointerDown` handler feeds all three recognizers
- [ ] Add keyboard accessibility (AC: 1)
  - [ ] Pill `tabIndex={0}` and `role="button"`, `aria-expanded`
  - [ ] Enter/Space on pill = click to expand
  - [ ] Escape = collapse
  - [ ] Focus trap inside expanded view

## Dev Notes

- Use pointer events (not touch-specific) for Wayland compatibility
- Component: `dyd-ui/src/components/Pill.tsx` — Gesture handling wraps pill content
- Context menu: `dyd-ui/src/components/ContextMenu.tsx` (new component)
- Gesture recognizer logic can be a custom hook: `useGestureHandler`
- Refer to EPIC-02-PILL-RENDERING.md §Feature 2.3 for detailed gesture specs
- Dormant state has `pointer-events: none` or equivalent for input passthrough

### Project Structure Notes

- Keep gesture logic in a custom hook separate from rendering
- Context menu should use Portal for proper z-index stacking

### References

- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#Feature-23-Gesture-Handling]
- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#US-231-Click-to-Expand]
- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#US-232-Swipe-to-Dismiss]
- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#US-233-Long-Press-Context-Menu]
- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#US-234-Gesture-Conflict-Resolution]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List
