# Story 2.5: Hover & Focus States

Status: ready-for-dev

## Story

As an end user,
I want the Dynamic Island to provide subtle visual feedback on hover (mouse glow) and keyboard focus (visible outline),
so that I know the pill is interactive and can navigate it with keyboard.

## Acceptance Criteria

1. Hover on active/expanded pill shows subtle glow (brightness +10% or 2px white shadow at 30% opacity, 150ms fade-in)
2. Hover only on devices with hover capability (not touch — use `@media (hover: hover)`)
3. Dormant pill does not respond to hover
4. Keyboard focus shows 2px solid white outline with 2px offset, rounded to match pill
5. Focus ring uses `:focus-visible` (not `:focus`) — no outline on mouse click
6. Interactive content inside expanded view has its own hover states
7. All hover/focus transitions are 150-200ms

## Tasks / Subtasks

- [ ] Implement hover glow effect (AC: 1, 2, 3)
  - [ ] CSS: `.pill:hover { box-shadow: 0 0 8px rgba(255,255,255,0.3); transition: box-shadow 150ms ease; }`
  - [ ] Wrap in `@media (hover: hover)` to exclude touch devices
  - [ ] Alternative: Motion's `whileHover` prop for animated hover
  - [ ] Subtle scale (1.02) + brightness filter as additional option
  - [ ] Only on active/expanded states (dormant has `pointer-events: none`)
- [ ] Implement focus outline for keyboard navigation (AC: 4, 5)
  - [ ] CSS: `.pill:focus-visible { outline: 2px solid white; outline-offset: 2px; border-radius: inherit; }`
  - [ ] Pill `tabIndex={0}`, `role="button"`, `aria-expanded`
  - [ ] Focus ring only on `:focus-visible` (not `:focus`)
  - [ ] Click outside removes focus
- [ ] Implement hover states on expanded content (AC: 6)
  - [ ] Buttons: `hover:bg-white/10`, `hover:text-white`, `hover:scale-105`
  - [ ] Clickable text: hover underline or color change
  - [ ] Disabled elements: no hover effect
  - [ ] Consistent transition speed: 150ms
- [ ] Implement focus trap inside expanded view (AC: 4)
  - [ ] Tab cycles through focusable elements inside expanded view
  - [ ] Shift+Tab on first element moves focus to last (and vice versa)
  - [ ] Escape closes expanded view, returns focus to pill

## Dev Notes

- CSS-based hover/focus is preferred over JS for performance
- Tailwind classes can be used: `hover:shadow-glow`, `focus-visible:ring-2`, etc.
- If Tailwind config needs extending for glow shadow, update `tailwind.config.js`
- Refer to EPIC-02-PILL-RENDERING.md §Feature 2.5 for detailed specs
- Dormant state: `pointer-events: none` disables all hover/focus

### Project Structure Notes

- CSS classes can be in `dyd-ui/src/styles/` or inline with Tailwind
- Focus trap can be a utility hook: `useFocusTrap`

### References

- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#Feature-25-Hover--Focus-States]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List
