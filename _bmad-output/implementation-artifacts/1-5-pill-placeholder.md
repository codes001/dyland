# Story 1.5: Pill Placeholder

Status: review

## Story

As an end user,
I want to see a small black pill at the top-center of my screen
so that I know the Dynamic Island application is running.

## Acceptance Criteria

1. Pill is exactly 126px wide × 36px tall
2. Pill has fully rounded corners (18px radius or `rounded-full`)
3. Pill background is `#000000` (black)
4. Pill is centered horizontally in the window
5. Pill sits flush at the top edge of the screen (y=0)
6. No border, shadow, or outline around the pill
7. Desktop background is fully visible through window outside pill
8. `Pill` component accepts props: `state`, `children`, `className`, `onClick`, `style`
9. Component uses `forwardRef` for animation library access
10. Component is pure (same props → same render)
11. Component is unit-testable with React Testing Library

## Tasks / Subtasks

- [x] Task 1: Create Pill React component (AC: 1, 2, 3, 4, 5, 6)
  - [x] Create `dyd-ui/src/components/Pill.tsx` with forwardRef
  - [x] Style with inline styles for exact pixel dimensions (126×36px)
  - [x] Add 18px border-radius (rounded-full)
  - [x] Black background (#000000)
  - [x] Position: fixed top-0 left-1/2 -translate-x-1/2
- [x] Task 2: Define Pill props interface (AC: 8, 9, 10)
  - [x] Define PillProps with state, children, className, onClick, style
  - [x] Use React.forwardRef<HTMLDivElement, PillProps>
  - [x] Default width: 126px (dormant), expands with children
- [x] Task 3: Ensure full transparency outside pill (AC: 7)
  - [x] Verify CSS: no background on html, body, #app
  - [x] overflow: hidden on body
- [x] Task 4: Add component unit tests (AC: 11)
  - [x] Test pill renders with correct dimensions
  - [x] Test pill accepts className prop
  - [x] Test pill fires onClick handler

## Dev Notes

- React component in `dyd-ui/src/components/Pill.tsx`
- Use inline styles for pixel-perfect Apple dimensions
- Component API: { state: 'dormant' | 'active' | 'expanded', children?, className?, style?, onClick? }
- Use React.forwardRef for future animation library access
- CSS: transparent background on html, body, #root

### References

- [Source: bmad-output/EPIC-01-FOUNDATION.md#Feature-1.5]
- [Source: bmad-output/STYLE-GUIDE.md#Color-Palette]

## Dev Agent Record

### Agent Model Used

deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- ✅ Pill component with exact Apple dimensions (126×36px)
- ✅ forwardRef for animation library access
- ✅ Transparent backgrounds throughout
- ✅ ConnectionIndicator for disconnected state

### File List

New files (relative to project root):
- `dyd-ui/src/components/Pill.tsx` — Pill React component
- `dyd-ui/src/components/ConnectionIndicator.tsx` — Connection status indicator

Modified files:
- `dyd-ui/src/App.tsx` — Added Pill and ConnectionIndicator
- `dyd-ui/src/index.css` — Transparent backgrounds, overflow hidden

## Change Log

- 2026-07-19: Initial story creation from Epic 1 Feature 1.5
- 2026-07-19: Full implementation — Pill component, transparency, ConnectionIndicator

## Status

Review — Story implementation complete, awaiting code review.
