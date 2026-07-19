# Story 2.2: Smooth Animations

Status: ready-for-dev

## Story

As an end user,
I want the Dynamic Island to animate with natural, physics-based motion using spring animations, content transitions, and a blurred backdrop,
so that the pill feels alive, premium, and responsive at 60fps.

## Acceptance Criteria

1. Pill width/height transitions use spring animation (stiffness:300-350, damping:25-30, mass:0.8)
2. Content inside the pill animates in/out with slide+fade (entry: 200ms slide up, exit: 150ms slide down)
3. Expanded view has smooth backdrop blur transition (20px blur, 300ms fade)
4. All animations run at 60fps on integrated Intel GPU
5. Animations use only `transform` and `opacity` (no layout-triggering properties)
6. Content animation is staggered 50ms after pill size animation

## Tasks / Subtasks

- [ ] Implement spring-based size transitions (AC: 1, 4, 5)
  - [ ] Use Motion's `motion.div` with `animate={{ scaleX, scaleY }}` and spring config
  - [ ] Apple-like spring: `stiffness: 350, damping: 25, mass: 0.8`
  - [ ] Expanding overshoots target by ~5% and settles (spring bounce)
  - [ ] Collapsing has slight follow-through
  - [ ] Pre-allocate GPU layers: `will-change: transform`
  - [ ] Use `transform: scale()` not `width`/`height`
  - [ ] Measure pill width dynamically with `useRef` + `ResizeObserver`
- [ ] Implement content entry/exit animations (AC: 2, 6)
  - [ ] New content: `initial={{ y: 10, opacity: 0 }}`, `animate={{ y: 0, opacity: 1 }}` (200ms)
  - [ ] Dismissed content: `exit={{ y: -10, opacity: 0 }}` (150ms)
  - [ ] Content change during display → cross-fade (new slides over old)
  - [ ] Stagger content animation 50ms after pill size animation
  - [ ] Use `AnimatePresence` with `mode="popLayout"`
- [ ] Implement background blur on expanded view (AC: 3)
  - [ ] CSS `backdrop-filter: blur(20px)` on expanded container
  - [ ] `-webkit-backdrop-filter: blur(20px)` WebKit fallback
  - [ ] Animate blur in with Motion (300ms fade)
  - [ ] Detect support: `CSS.supports('backdrop-filter', 'blur(1px)')`
  - [ ] Fallback: `background: rgba(0, 0, 0, 0.6)`
  - [ ] Test performance on integrated GPU, reduce blur radius if needed
- [ ] Implement 60fps performance optimization (AC: 4, 5)
  - [ ] Use only `transform` and `opacity` for animations
  - [ ] Apply `will-change: transform, opacity` on animated elements
  - [ ] Use Motion's `layoutDependency` to minimize re-renders
  - [ ] No layout thrashing during animations
  - [ ] Profile with browser DevTools Performance tab

## Dev Notes

- Motion library (`motion` package) should be in dependencies from Epic 1 scaffolding
- Component files: `dyd-ui/src/components/Pill.tsx`, `dyd-ui/src/components/ExpandedView.tsx`
- Animation config constants should be defined in a shared config file e.g. `dyd-ui/src/config/animation.ts`
- Refer to EPIC-02-PILL-RENDERING.md §Feature 2.2 for detailed spring constants and easing values
- Refer to SYSTEM-DESIGN.md §State Transition Effects for the authoritative transition table
- Avoid animating `width`, `height`, `top`, `left`, `margin`, `padding` — always use `transform`

### Project Structure Notes

- Follow Epic 1 React component patterns (function components, Tailwind classes, TypeScript)
- Animation constants should be centralized, not inlined

### References

- [Source: bmad-output/EPIC-02-PILL-RENDERING.md#Feature-22-Smooth-Animations]
- [Source: bmad-output/SYSTEM-DESIGN.md#State-Transition-Effects]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List
