# Epic 2: The Pill — Rendering & Interaction

## Objective
Transform the static pill placeholder into a fully animated, interactive Dynamic Island with multiple states (dormant, active, expanded), smooth physics-based transitions, gesture handling, and content queuing. After this Epic, the pill feels alive and responsive.

## Business Value
This is the core user-facing feature. The quality of these animations and interactions determines whether the app feels "premium" or "toy." This Epic is the product's differentiator.

## Success Criteria
- Pill transitions smoothly between dormant, active, and expanded states at 60fps
- Content slides, fades, and scales with natural easing curves
- Click expands the island, swipe dismisses, long-press shows menu
- Multiple activities (notifications, media, etc.) queue and display sequentially
- Hover shows subtle feedback (glow, scale)
- Pill hides during fullscreen applications
- All animations are interruptible (responsive to new events mid-transition)

## Risks
- Motion library (Motion / React Spring) may introduce bundle size and performance overhead
- Physics-based animations require careful tuning — easy to make feel "wrong"
- Gesture handling on Wayland may have edge cases with touch input vs mouse
- Fullscreen detection on Wayland has no standard API
- Animation performance at 60fps on integrated GPUs (Intel, older AMD) requires optimization

## Dependencies
- Epic 1 (Foundation) — specifically the transparent overlay window and IPC

## Complexity
⭐⭐⭐⭐⭐ — Highest UX complexity. Requires animation expertise and extensive tuning.

---

## Feature 2.1: State Machine

**Description:** Define and implement the state machine for the Dynamic Island with four primary states: Dormant (minimal pill), Active (expanding/contracting with content), Expanded (full content view), and Hidden (obscured during fullscreen). State transitions must be deterministic and interruptible.

**Acceptance Criteria:** State machine implemented with clear transitions. States persist across content changes. Interruptible transitions (new event mid-animation cancels current animation and starts new one).

**Dependencies:** 1.5 (Pill component architecture)

**Notes:** Use XState or a lightweight custom state machine. States should be driven by events from the daemon (media starts, notification arrives) and user interactions (click, swipe).

### User Stories

#### US-2.1.1 — State Definitions
**As a** developer
**I want** four clearly defined states for the Dynamic Island
**so that** all visual rendering and behavior maps to a known state.

**Acceptance Criteria:**
- `dormant`: Minimal pill (126×36px), no content, no interaction area, input passthrough
- `active`: Slightly expanded (width grows to accommodate icon + 1-line text), shows current activity, interactive
- `expanded`: Full content view (wider, taller), shows detailed content, fully interactive
- `hidden`: Pill is off-screen or invisible, used during fullscreen apps
- Each state has a defined entry, exit, and duration

**Edge Cases:**
- State transitions can overlap (interrupt a mid-transition to go to another state)
- Dormant → active → dormant should not flash or flicker
- Hidden state should be reachable from any state

**Technical Notes:**
- Use a `useReducer` or Zustand store for state management
- TypeScript enum: `'dormant' | 'active' | 'expanded' | 'hidden'`
- State machine rules:
  - `dormant` → `active` (new activity)
  - `active` → `expanded` (user click)
  - `active` → `dormant` (activity dismissed/times out)
  - `expanded` → `active` (user clicks away or swipe down)
  - `*` → `hidden` (fullscreen detected)
  - `hidden` → `*` (prev state on fullscreen exit)

**Priority:** P0

#### US-2.1.2 — Deterministic Transitions
**As a** system
**I want** all state transitions to be deterministic with defined animation curves and durations
**so that** the behavior is predictable and testable.

**Acceptance Criteria:**
- Each transition has a defined: target state, animation duration, easing curve, content animation direction
- Transition table is documented and exhaustive
- Transition from state A to state B always produces the same visual result given the same input
- Animation timing is consistent (±16ms)

**Edge Cases:**
- Content changing during transition → interrupt current transition, start new one
- Rapid state changes (e.g., 10 notifications in 1 second) → queue or merge

**Technical Notes:**
- Transition table:
  | From | To | Duration | Easing | Content |
  |---|---|---|---|---|
  | dormant | active | 300ms | ease-out | Slide up + fade in |
  | active | dormant | 200ms | ease-in | Slide down + fade out |
  | active | expanded | 400ms | spring | Scale + expand |
  | expanded | active | 300ms | spring | Shrink + collapse |
- Use Motion's `transition` prop for animation configuration

**Priority:** P0

#### US-2.1.3 — Interruptible Animations
**As an** end user
**I want** animations to be interruptible
**so that** the island responds immediately to new events even during a transition.

**Acceptance Criteria:**
- New event received while animating → current animation is cancelled, new animation starts from current visual state
- No visual glitch or flash during interruption
- Interruption preserves the target content (doesn't show stale content)

**Edge Cases:**
- Multiple rapid interruptions → only the last event matters
- Interruption during physics-based spring animation → spring velocity is preserved (natural feel)

**Technical Notes:**
- Motion's `AnimatePresence` handles interruptible layout animations
- Use `layout` prop on Motion components for automatic FLIP animation
- For spring animations, preserve velocity on interrupt using `spring()` with `velocity` prop

**Priority:** P1

#### US-2.1.4 — Idle Timeout & Auto-Dismiss
**As an** end user
**I want** the active state to auto-dismiss back to dormant after a configurable timeout
**so that** the island returns to its minimal state when there is no ongoing activity.

**Acceptance Criteria:**
- Active state auto-dismisses after 4 seconds of no interaction or new events (configurable in settings)
- Timer resets on user interaction (hover, click)
- Timer resets on new event arrival
- No timeout when in expanded state (user explicitly opened it)
- Expanding the island cancels the timeout

**Edge Cases:**
- User is actively reading content when timeout fires → content should not disappear
- Multiple timers running simultaneously (race condition prevention)

**Technical Notes:**
- Use `useEffect` with `setTimeout`/`clearTimeout` pattern
- Store timer ID in a ref to prevent stale closure issues
- Configurable timeout value from settings store

**Priority:** P2

---

## Feature 2.2: Smooth Animations

**Description:** Implement all visual animations using Motion/React Spring with physics-based spring animations, FLIP layout transitions, and GPU-accelerated transforms. Every transition must feel natural and polished at 60fps.

**Acceptance Criteria:** Pill width/height transition uses spring animation. Content fades/slides with ease curves. Background blur applies smoothly. All animations run at 60fps on integrated GPU.

**Dependencies:** 2.1

**Notes:** Use Motion's `motion.div` with `layout` prop for automatic FLIP. Avoid animating `width`/`height` directly — prefer `scaleX`/`scaleY` or CSS `transform`.

### User Stories

#### US-2.2.1 — Spring-Based Size Transitions
**As an** end user
**I want** the pill to expand and collapse with a natural spring-like motion
**so that** it feels physically alive rather than mechanical.

**Acceptance Criteria:**
- Expanding animation overshoots target size by ~5% and settles back (spring bounce)
- Collapsing animation has slight follow-through
- Animation duration: ~400ms for expand, ~300ms for collapse
- Transition uses `transform: scaleX()` and `transform: scaleY()` (not layout-triggering properties)
- Animation is GPU-composited (uses `will-change: transform`)

**Edge Cases:**
- Rapid expand/collapse → spring velocity should carry over correctly
- Very small width changes → should still animate smoothly (no snapping)

**Technical Notes:**
- Use Motion's `motion.div` with `animate={{ scaleX, scaleY }}` and `transition={{ type: 'spring', stiffness: 300, damping: 25 }}`
- Apple-like spring: `stiffness: 350, damping: 25, mass: 0.8`
- Alternatively, use `layout` prop with `layoutTransition` for automatic size animation
- Pre-allocate GPU layers: `style={{ willChange: 'transform' }}`
- Measure pill width dynamically with `useRef` + `ResizeObserver`

**Priority:** P0

#### US-2.2.2 — Content Entry/Exit Animations
**As an** end user
**I want** content inside the pill to animate in and out smoothly
**so that** the transition between different activities feels fluid.

**Acceptance Criteria:**
- New content slides up and fades in from below (upward direction)
- Dismissed content slides down and fades out
- Content change while existing content is showing → cross-fade (new content slides over old)
- Duration: 200ms for entry, 150ms for exit
- Content animation is slightly delayed relative to pill size change (staggered)

**Edge Cases:**
- Empty content → pill shows empty state with no animation
- Very long text → should truncate, animation still plays

**Technical Notes:**
- Use Motion's `AnimatePresence` with `mode="popLayout"` for enter/exit animations
- Wrap content in `motion.div` with `initial={{ y: 10, opacity: 0 }}`, `animate={{ y: 0, opacity: 1 }}`, `exit={{ y: -10, opacity: 0 }}`
- Stagger: delay content animation by 50ms after pill size animation starts

**Priority:** P0

#### US-2.2.3 — Background Blur & Backdrop
**As an** end user
**I want** the expanded island to have a blurred background effect
**so that** it visually separates from the desktop content behind it.

**Acceptance Criteria:**
- Expanded state shows a subtle backdrop blur behind the content
- Blur radius: ~20px (Apple-like frosted glass effect)
- Blur transitions in and out smoothly (300ms)
- Blur does not cause jank or frame drops
- Fallback: semi-transparent dark background if backdrop blur is not supported

**Edge Cases:**
- Compositor may not support `backdrop-filter: blur()` on transparent windows
- Performance impact on integrated GPUs → reduce blur radius or disable
- Multi-monitor: blur captures correct screen content

**Technical Notes:**
- CSS: `backdrop-filter: blur(20px)` on the expanded content container
- `-webkit-backdrop-filter: blur(20px)` for WebKit fallback
- Detect support: `CSS.supports('backdrop-filter', 'blur(1px)')`
- Fallback: `background: rgba(0, 0, 0, 0.6)` when backdrop blur unsupported
- Wrap in `motion.div` to animate the blur value

**Priority:** P1

#### US-2.2.4 — Performance Optimization for Animations
**As an** end user
**I want** all animations to run at a smooth 60fps
**so that** the experience feels premium and responsive.

**Acceptance Criteria:**
- Animations run at 60fps on integrated Intel UHD Graphics 620+ (or equivalent)
- No layout thrashing during animations (Layout performance score: 95+ on DevTools)
- JavaScript main thread is not blocked during animations
- Animations continue at 60fps with multiple events arriving
- Memory usage does not increase during extended animation sequences

**Edge Cases:**
- Low battery mode → consider reducing animation quality
- Very high DPI → ensure GPU compositing is not overwhelmed
- Multiple monitors → animations should not affect non-primary monitor performance

**Technical Notes:**
- Use `transform` and `opacity` only for animations (compositor-friendly)
- Avoid animating `width`, `height`, `top`, `left`, `margin`, `padding`
- Use `will-change: transform, opacity` on animated elements
- Use Motion's `layoutDependency` to minimize re-renders
- Profile with Chrome DevTools Performance tab and GPU timeline
- Monitor with `requestAnimationFrame` timing checks in development mode

**Priority:** P1

---

## Feature 2.3: Gesture Handling

**Description:** Implement gesture recognition for click (expand), swipe (dismiss), and long-press (context menu). Gestures should feel native and responsive on both mouse and touch input.

**Acceptance Criteria:** Click expands to content view. Swipe down dismisses current activity. Long-press shows context menu. Gestures work on mouse and touch. No gesture conflicts.

**Dependencies:** 2.1

**Notes:** Use Gestify or custom gesture handling with pointer events. Avoid adding large gesture libraries. Prefer pointer events over touch-specific events for Wayland compatibility.

### User Stories

#### US-2.3.1 — Click to Expand
**As an** end user
**I want** to click the active pill to expand it into the full content view
**so that** I can see details of the current activity.

**Acceptance Criteria:**
- Click on dormant pill → no action (dormant is not interactive)
- Click on active pill → transitions to expanded state
- Click outside the expanded island → returns to active state
- Click on content links/buttons inside expanded view → triggers action, does not collapse

**Edge Cases:**
- Double-click → should not cause issues (first click expands, second click lands on content)
- Click on pill during animation → should queue or interrupt

**Technical Notes:**
- Use `onClick` on the pill container
- For "click outside": use a transparent overlay behind the expanded island or detect `click` events on window
- `event.stopPropagation()` on interactive content elements

**Priority:** P0

#### US-2.3.2 — Swipe to Dismiss
**As an** end user
**I want** to swipe the active pill downward to dismiss the current activity
**so that** I can quickly clear notifications or stop media display.

**Acceptance Criteria:**
- Swipe down on active pill → dismisses current activity
- Swipe distance threshold: 30px
- Swipe velocity threshold: 200px/s
- During swipe, pill follows finger/mouse position (rubber-banding on exceeding threshold)
- After release below threshold → pill snaps back to active state (animated)
- After release above threshold → pill dismisses content, returns to dormant (animated)

**Edge Cases:**
- Swipe up → should not dismiss (or should do nothing)
- Swipe during expanded state → collapse to active (not to dormant)
- Touch vs mouse swipe → both handled identically

**Technical Notes:**
- Use pointer events: `onPointerDown`, `onPointerMove`, `onPointerUp`
- Track: startY, currentY, startTime
- Formula for rubber-banding (beyond threshold): `offset = threshold + (distance - threshold) * 0.3`
- Lock scroll/swipe direction to vertical only
- After release: if velocity > 200px/s OR distance > 30px → dismiss, else → snap back

**Priority:** P1

#### US-2.3.3 — Long-Press Context Menu
**As an** end user
**I want** to long-press the pill to see a context menu with quick actions
**so that** I can access common actions without expanding the island.

**Acceptance Criteria:**
- Long-press on active/expanded pill → shows context menu after 500ms
- Context menu appears as a small popup below the pill
- Menu options depend on current activity (e.g., "Skip" for media, "Dismiss" for notification, "Settings")
- Haptic-like visual feedback (pulse/grow animation) at 300ms to confirm press
- Release before 500ms → no action (regular click/gesture takes priority)

**Edge Cases:**
- Long-press then drag → should treat as swipe, not long-press
- Context menu open → pill should not auto-dismiss
- Multiple long-presses → only one context menu

**Technical Notes:**
- Use `onPointerDown` + `setTimeout(500ms)` to detect hold
- Cancel timeout on `onPointerUp`, `onPointerMove` (if moved > 10px), or `onPointerCancel`
- Visual feedback: scale up to 1.05 at 300ms, then hold until menu shows
- Context menu is a separate React component positioned via floating UI

**Priority:** P2

#### US-2.3.4 — Gesture Conflict Resolution
**As a** system
**I want** gesture recognition to resolve conflicts between click, swipe, and long-press
**so that** user intent is always correctly interpreted.

**Acceptance Criteria:**
- Click: pointerDown → pointerUp with < 10px movement and < 300ms duration
- Swipe: pointerDown → pointerMove > 10px vertical → pointerUp
- Long-press: pointerDown → 500ms hold with < 10px movement → pointerUp
- These three gesture recognizers do not interfere with each other
- One gesture firing cancels the other two

**Edge Cases:**
- Accessibility: keyboard equivalents for all gestures

**Technical Notes:**
- Implement a simple gesture recognizer state machine per pointer event sequence:
  ```
  PENDING → (move > 10px) → SWIPE
  PENDING → (timeout 500ms) → LONG_PRESS
  PENDING → (pointerUp < 300ms) → CLICK
  ```
- Use a single `onPointerDown` handler that feeds into all three recognizers
- First recognizer to match cancels the others

**Priority:** P2

---

## Feature 2.4: Multi-Activity Queue

**Description:** Implement a queue system that manages multiple concurrent activities (notifications, media, timers, etc.) and displays them sequentially in the Dynamic Island. The queue handles prioritization, stacking, and cycling.

**Acceptance Criteria:** Multiple activities queue up. Active activity displays for 4 seconds, then next in queue displays. Priority system (calls > media > notifications). User can cycle through queue via gestures.

**Dependencies:** 2.1, 2.2

**Notes:** Queue is managed in the daemon, current state is synced to UI over IPC. Prioritization rules are configurable.

### User Stories

#### US-2.4.1 — Activity Queue Data Structure
**As a** system
**I want** an activity queue managed in the daemon
**so that** multiple concurrent activities are handled in priority order.

**Acceptance Criteria:**
- Queue supports: push (new activity), pop (remove current), peek (view next), remove (by ID)
- Queue is ordered by priority then FIFO within same priority
- Priority levels: CRITICAL (incoming call) > HIGH (timer/alarm) > MEDIUM (media) > LOW (notification) > INFO (system status)
- Queue capacity: max 20 items (oldest LOW items removed when full)
- Queue state is synced to UI over IPC

**Edge Cases:**
- Empty queue → pill returns to dormant state
- All items same priority → FIFO order
- Duplicate activity IDs → ignore (deduplicate by ID)

**Technical Notes:**
- Rust `BinaryHeap` with custom `Activity` ordering by priority + timestamp
- Activity struct:
  ```rust
  struct Activity {
      id: Uuid,
      source: ActivitySource,
      priority: Priority,
      timestamp: Instant,
      content: ActivityContent,
      ttl: Option<Duration>,  // auto-expire
  }
  ```
- Sync: push entire queue state to UI on any change (or push delta events)

**Priority:** P0

#### US-2.4.2 — Sequential Display with Cycling
**As an** end user
**I want** the Dynamic Island to show one activity at a time and cycle through the queue
**so that** I can see all pending activities without them overlapping.

**Acceptance Criteria:**
- Current activity displays for 4 seconds (or until user interacts)
- After timeout → next activity in queue displays with slide transition
- After last activity → return to dormant
- User can cycle forward (swipe up) or backward (swipe down, unless dismiss)
- Cycling does not remove activities from queue (unlike dismiss)

**Edge Cases:**
- Single activity → no cycling, stays active until dismissed or times out
- Queue changes while displaying → current activity finishes, then next is recalculated
- Very high-priority item arrives → preempt current display (if lower priority)

**Technical Notes:**
- Timer: 4s per activity, stored in daemon
- "Next" button/gesture: sends IPC command `cycle_next`
- "Previous" gesture: sends IPC command `cycle_prev`
- Activity display state managed in daemon, synced to UI

**Priority:** P1

#### US-2.4.3 — Activity Prioritization
**As an** end user
**I want** high-priority activities (calls, timers) to preempt lower-priority ones
**so that** I don't miss important events.

**Acceptance Criteria:**
- Incoming call (CRITICAL) → immediately shows, interrupts current display
- Timer/alarm (HIGH) → shows after current activity finishes (or immediately if current is MEDIUM or lower)
- Media change (MEDIUM) → shows after current notification
- P0 activities are not preempted by lower priority
- Priority preemption is configurable in settings

**Edge Cases:**
- Two CRITICAL events arrive simultaneously → first-come-first-served, second queues
- HIGH event arrives while another HIGH is showing → cycle through normally (already same priority)

**Technical Notes:**
- Preemption check when pushing to queue:
  - If new activity priority > current displayed activity priority → interrupt
  - If new activity priority == current priority → queue behind
  - If new activity priority < current priority → add to queue, no preemption
- Interruption animation: current slides out, new slides in (200ms)
- Interrupted activity remains in queue (not dismissed)

**Priority:** P1

#### US-2.4.4 — Queue Persistence & Restoration
**As a** system
**I want** the activity queue to persist through daemon restarts
**so that** ongoing activities (like timers) survive a daemon crash.

**Acceptance Criteria:**
- Queue state is persisted to disk at `$XDG_STATE_HOME/dyd/queue.json`
- Queue is restored on daemon startup
- Expired items (past TTL) are not restored
- Corruption of queue file → start with empty queue, log warning

**Edge Cases:**
- State directory does not exist → create it
- Concurrent write during crash → atomic write with rename

**Technical Notes:**
- Use `serde_json` with `write` + `sync_all` + atomic rename for writes
- Write on every queue mutation (debounced: max 1 write per 500ms)
- Read on daemon startup before accepting new activities

**Priority:** P2

---

## Feature 2.5: Hover & Focus States

**Description:** Implement subtle visual feedback for hover (mouse over) and focus (keyboard) states. The pill should respond to user presence without being distracting.

**Acceptance Criteria:** Hover shows subtle glow or brightness increase. Focus shows outline for keyboard navigation. Feedback is minimal and tasteful.

**Dependencies:** 2.1

**Notes:** Must not interfere with touch interactions (hover doesn't exist on touch). Use CSS `:hover` with `@media (hover: hover)` query.

### User Stories

#### US-2.5.1 — Hover Glow Effect
**As an** end user
**I want** the pill to subtly glow when I hover over it with my mouse
**so that** I know it is interactive.

**Acceptance Criteria:**
- Hover effect: slight brightness increase (+10%) or subtle white glow (2px spread, 30% opacity)
- Effect fades in over 150ms, fades out over 200ms
- Only applies to active/expanded states (dormant does not respond to hover)
- Effect only works on devices with hover capability (not touch)
- The effect is subtle — should not be distracting

**Edge Cases:**
- Touch devices → no hover state (use active state instead)
- Hover over interactive content inside expanded view → separate hover states for those elements

**Technical Notes:**
- CSS: `.pill:hover { box-shadow: 0 0 8px rgba(255,255,255,0.3); transition: box-shadow 150ms ease; }`
- `@media (hover: hover) { ... }` to exclude touch devices
- Or use Motion's `whileHover` prop for animated hover
- Consider using a subtle scale (1.02) + brightness filter

**Priority:** P1

#### US-2.5.2 — Focus Outline for Keyboard Navigation
**As an** end user
**I want** the pill to show a visible focus indicator when navigated with the keyboard
**so that** the application is accessible to keyboard users.

**Acceptance Criteria:**
- Pill is focusable via Tab key
- Focus indicator: 2px solid white outline, 2px offset
- Focus ring appears only on keyboard focus (not on mouse click)
- Focus ring matches the pill's rounded corners
- Clicking outside pill removes focus

**Edge Cases:**
- Focus ring should not appear on mouse click (use `:focus-visible` not `:focus`)
- Expanded state internal elements should be independently focusable

**Technical Notes:**
- CSS: `.pill:focus-visible { outline: 2px solid white; outline-offset: 2px; border-radius: inherit; }`
- Make pill `tabIndex={0}` and `role="button"`
- Use `aria-expanded` on pill to communicate state to screen readers
- Trap focus inside expanded view (for keyboard navigation)

**Priority:** P2

#### US-2.5.3 — Hover State on Expanded Content
**As an** end user
**I want** buttons and interactive elements inside the expanded island to have hover states
**so that** I know what is clickable.

**Acceptance Criteria:**
- Buttons inside expanded view show hover effect (slightly brighter or color change)
- Clickable text (like notification action buttons) show hover underline or color change
- Hover effects follow the same design language (subtle, tasteful)
- Disabled elements do not show hover effects

**Edge Cases:**
- Touch devices → show no hover, but active/tap state instead

**Technical Notes:**
- Standard Tailwind hover classes: `hover:bg-white/10`, `hover:text-white`, `hover:scale-105`
- Consistent transition speed: 150ms

**Priority:** P2

---

## Feature 2.6: Fullscreen Detection

**Description:** Detect when a fullscreen application is active and hide the Dynamic Island to avoid interference. Restore to previous state when fullscreen ends.

**Acceptance Criteria:** Pill hides during fullscreen video/game/app. Pill reappears when fullscreen ends. Detection works on Wayland.

**Dependencies:** 2.1

**Notes:** This is technically challenging on Wayland. No standard signal for fullscreen transitions. Multiple fallback strategies needed.

### User Stories

#### US-2.6.1 — Wayland Fullscreen Detection via Active Window
**As a** system
**I want** to detect when a fullscreen window becomes active
**so that** the Dynamic Island can hide itself.

**Acceptance Criteria:**
- When a window enters fullscreen mode, system detects within 500ms
- Detection methods (in order of preference):
  1. D-Bus `org.gnome.Shell.Introspect` for GNOME-specific fullscreen state
  2. `wlr-foreign-toplevel-management` protocol for wlroots-based compositors
  3. Polling `xwininfo`/`xdotool` (X11 fallback, not primary)
- Pill transitions to hidden state with smooth slide-up animation (100ms)
- Only hides when fullscreen window is on the same monitor as the pill

**Edge Cases:**
- Multiple monitors → fullscreen window on monitor 2 should not hide pill on monitor 1
- Picture-in-picture mode (not fullscreen) → should not trigger hide
- Fullscreen browser video → should trigger hide

**Technical Notes:**
- Use `zbus` to call `org.gnome.Shell.Introspect.GetWindows()` and check `fullscreen` property
- For wlroots: `zwlr_foreign_toplevel_management_v1` protocol
- Poll interval: 2 seconds (not critical for real-time, but responsive enough)
- Cache fullscreen state to avoid repeated D-Bus calls

**Priority:** P1

#### US-2.6.2 — Fullscreen Exit Restoration
**As an** end user
**I want** the Dynamic Island to reappear when I exit fullscreen mode
**so that** I don't have to manually restart it.

**Acceptance Criteria:**
- When last fullscreen window exits fullscreen, pill returns to its previous state (dormant or active)
- Restoration animation: slide-down (100ms), then continue with previous state
- If activity queue had items during hidden period, they are re-shown (oldest first)
- Restoration happens within 500ms of fullscreen exit

**Edge Cases:**
- User closes fullscreen window (Alt+F4) → window destroyed, no fullscreen-exit event → detect via active window change
- Multiple fullscreen windows (e.g., dual-monitor setup) → pill stays hidden until all exit fullscreen

**Technical Notes:**
- Track fullscreen window count on each monitor
- Restoration: when count reaches 0, trigger state restore
- For window destruction without explicit fullscreen-exit: poll active windows, check if any remain fullscreen

**Priority:** P1

#### US-2.6.3 — Manual Override
**As an** end user
**I want** the ability to manually override fullscreen hiding behavior
**so that** I can keep the Dynamic Island visible if I want to.

**Acceptance Criteria:**
- "Keep visible during fullscreen" toggle in settings
- When enabled, pill does not hide during fullscreen
- Toggle is remembered across sessions
- Visual note: pill may overlap with game/video UI when enabled

**Edge Cases:**
- Toggle changes while in fullscreen → apply immediately

**Technical Notes:**
- Setting stored in daemon config file
- Toggle event sent over IPC to daemon → daemon stops sending hide/show commands
- UI still listens for fullscreen events but ignores them when setting is on

**Priority:** P3
