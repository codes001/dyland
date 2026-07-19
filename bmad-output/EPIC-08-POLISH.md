# Epic 8: Polish & Performance

## Objective
Refine every aspect of the Dynamic Island to achieve a premium, Apple-quality feel. Optimize performance to meet the targets: startup <150ms, idle RAM <100MB, idle CPU near zero, animations at 60fps.

## Business Value
Polish is what separates a professional application from a hobby project. Users form their opinion of quality within the first seconds of use. Meeting performance targets ensures the app is usable on a wide range of hardware.

## Success Criteria
- Startup time measured from daemon launch to UI being interactive: <150ms
- Idle RAM (no activities): <100MB total (daemon + UI)
- Idle CPU: <1% on an i5-1240P or equivalent
- Animations: consistently 60fps (no dropped frames)
- No memory leaks over 24h of use
- App icon and desktop file properly installed
- Error states handled gracefully

## Risks
- Tauri + WebView baseline RAM is ~40-60MB, leaving only 40-60MB for application state
- Some D-Bus calls may block if services are slow
- 60fps animations on integrated GPUs may require aggressive optimization
- Wayland compositor differences may affect performance

## Dependencies
- Epics 1-7 must be largely complete (polish is applied to existing features)

## Complexity
⭐⭐⭐⭐⭐ — High. Optimization work is open-ended and requires extensive profiling.

---

## Feature 8.1: Animation Polish

**Description:** Tune all animations to match Apple's "designed feel" — subtle, purposeful, and physically plausible. Review every transition for timing, easing, and visual perfection.

**Acceptance Criteria:** All animations reviewed and tuned. Easing curves documented. Spring parameters documented. No jarring or abrupt transitions. Animation durations and curves match Apple's HIG where applicable.

**Dependencies:** 2.2

**Notes:** This is a refinement pass, not a rewrite. Small adjustments to timing, easing, and stagger can dramatically improve feel.

### User Stories

#### US-8.1.1 — Animation Audit
**As a** developer
**I want** to audit every animation in the application for timing, easing, and visual quality
**so that** the feel is consistent and polished.

**Acceptance Criteria:**
- Every animation transition documented with: trigger, duration, easing, animating properties
- Review against Apple's HIG animation guidelines:
  - Spring animations for interactive transitions (expand/collapse)
  - Ease-in-out for automated transitions (queue cycling)
  - Ease-out for content entry
  - Ease-in for content exit
- Adjust spring parameters:
  - Expand: `stiffness: 400, damping: 28, mass: 0.8` (snappy, slight overshoot)
  - Collapse: `stiffness: 350, damping: 30, mass: 0.8` (controlled, no bounce)
- Adjust duration: all animations should complete within 200-500ms

**Edge Cases:**
- Animation that feels "wrong" → iterate on parameters
- Consistency across similar transitions → ensure same parameters

**Technical Notes:**
- Create an animation design doc (or section in architecture docs)
- Use Motion's `spring()` utility for consistent spring configs
- Shared animation config in TypeScript:
  ```typescript
  export const ANIMATION = {
      expand: { type: 'spring', stiffness: 400, damping: 28, mass: 0.8 },
      collapse: { type: 'spring', stiffness: 350, damping: 30, mass: 0.8 },
      contentEnter: { duration: 0.2, ease: 'easeOut' },
      contentExit: { duration: 0.15, ease: 'easeIn' },
  } as const
  ```

**Priority:** P1

#### US-8.1.2 — Staggered & Coordinated Animations
**As an** end user
**I want** animations to feel coordinated and overlapping, not mechanical
**so that** the island feels alive.

**Acceptance Criteria:**
- Expand sequence:
  1. Pill width expands (0ms, 300ms spring)
  2. Background blur fades in (delay: 100ms, duration: 200ms)
  3. Content slides up (delay: 150ms, duration: 200ms)
- Collapse sequence:
  1. Content slides down (0ms, 150ms)
  2. Background blur fades out (delay: 50ms, duration: 150ms)
  3. Pill width collapses (delay: 100ms, duration: 250ms)
- Stagger delays are consistent across all similar transitions
- Overlapping animations (next starts before previous finishes)

**Edge Cases:**
- Interrupted animation → stagger restarts from current position

**Technical Notes:**
- Use Motion's `transition` with `delay` on each child element
- `AnimatePresence` with staggered children using `variants`

**Priority:** P2

#### US-8.1.3 — Haptic-like Visual Feedback
**As an** end user
**I want** subtle visual feedback when I interact with the island
**so that** the interaction feels acknowledged.

**Acceptance Criteria:**
- On click/tap: pill scales to 1.02 for 100ms, then springs back
- On long-press (at 300ms): pill pulses once (scale 1.03 → 1.0, 200ms)
- On swipe: pill follows finger position with slight drag resistance
- On action execution (play/pause): button has a quick scale pulse (1.1 → 1.0, 80ms)

**Edge Cases:**
- Reduced motion mode → skip all haptic feedback

**Technical Notes:**
- Motion's `whileTap` for click feedback
- Motion's `whileHover` for hover feedback (scale: 1.02)
- Use `onPointerDown` with setTimeout for long-press pulse

**Priority:** P2

---

## Feature 8.2: Startup Performance

**Description:** Optimize the startup sequence to achieve <150ms from launch to interactive UI. Profile and optimize the critical path.

**Acceptance Criteria:** Daemon starts, connects D-Bus, opens IPC socket, UI connects, renders pill — all within 150ms on an SSD-equipped laptop. Measured from process start to first frame rendered.

**Dependencies:** 1.2, 1.3, 1.4

**Notes:** This is a stretch goal. Even 300ms is acceptable for v1, but we should push toward 150ms.

### User Stories

#### US-8.2.1 — Startup Profiling
**As a** developer
**I want** to profile the startup sequence end-to-end
**so that** I can identify and eliminate bottlenecks.

**Acceptance Criteria:**
- Instrumented startup with trace points at:
  - T0: daemon binary starts
  - T1: tokio runtime initialized
  - T2: D-Bus connection established
  - T3: IPC socket listening
  - T4: UI process spawned
  - T5: WebView initialized
  - T6: React app mounted
  - T7: first frame rendered (pill visible)
- Trace output to log on startup
- Target: T7 - T0 < 150ms

**Edge Cases:**
- Cold start (no cache) vs warm start (cached WebView)
- Different storage (NVMe vs SATA SSD vs HDD)

**Technical Notes:**
- Use `tracing` spans with timestamps for instrumentation
- Profile with `perf` for daemon, Chrome DevTools for UI
- Bottlenecks expected: WebView initialization (50-100ms), D-Bus connection (10-30ms)

**Priority:** P2

#### US-8.2.2 — Lazy Initialization
**As a** system
**I want** to defer non-critical initialization to after the first frame
**so that** the startup time is minimized.

**Acceptance Criteria:**
- Critical path (blocking render):
  1. Tokio runtime init
  2. IPC socket binding
  3. UI process spawn
  4. WebView init
  5. Pill render
- Deferred (after first frame):
  - D-Bus listeners (MPRIS, notifications, system status)
  - Plugin loading
  - Config read (if fast — config is small, this is on critical path)
  - Theme system init
  - Cache warmup
- Deferred initialization completes within 500ms after startup

**Edge Cases:**
- D-Bus listener starts after a notification arrives → notification will be missed (acceptable: first notification may not appear in island)

**Technical Notes:**
- In daemon: `tokio::spawn` for deferred tasks
- In UI: `React.lazy()` or `setTimeout(() => initNonCritical(), 0)` after first render

**Priority:** P2

---

## Feature 8.3: Runtime Performance & Memory

**Description:** Optimize runtime CPU and memory usage. Achieve near-zero idle CPU and <100MB total RAM.

**Acceptance Criteria:** Idle CPU <1%. Idle RAM <100MB (daemon + UI). No memory leaks over 24h. D-Bus event handling <1ms per event.

**Dependencies:** 1.2, 1.4, 8.2

**Notes:** Performance regression tests should be part of CI.

### User Stories

#### US-8.3.1 — CPU Profiling & Optimization
**As a** developer
**I want** to profile CPU usage in idle and active states
**so that** I can eliminate unnecessary CPU consumption.

**Acceptance Criteria:**
- Idle CPU: measure with `htop` / `perf` over 5 minutes
  - Daemon: <0.5% CPU on i5-1240P
  - UI: <0.5% CPU (WebView process)
- Active CPU (during animation/event processing): spikes allowed, but should return to idle within 1 second
- No polling loops — all event-driven
- D-Bus signal handlers are non-blocking

**Edge Cases:**
- High-frequency events (rapidly changing volume, fast song skipping) → debounce/throttle
- Many simultaneous D-Bus signals → use tokio tasks, not blocking calls

**Technical Notes:**
- Profile with `perf top`, `flamegraph-rs`, and `htop`
- Common issues: JSON serialization in hot paths, unnecessary re-renders, polling instead of signals
- UI: use React DevTools Profiler to identify re-render issues

**Priority:** P1

#### US-8.3.2 — Memory Profiling & Leak Detection
**As a** developer
**I want** to profile memory usage and detect leaks
**so that** the application remains stable over long sessions.

**Acceptance Criteria:**
- Measure with `heaptrack` or `valgrind`:
  - Baseline memory after startup
  - Memory after 24h of simulated usage (events, notifications, media changes)
  - Memory difference <5MB (no significant leak)
- Daemon heap: <30MB RSS
- UI + WebView: <70MB RSS
- Album art cache: <50MB on disk as configured
- Activity queue items are properly dropped when removed

**Edge Cases:**
- Memory pressure: system OOM → daemon should handle gracefully (reduce cache, clear queue)
- Long-running daemon (days/weeks) → monitor for gradual growth

**Technical Notes:**
- Use `heaptrack` on Linux for daemon memory profiling
- Chrome DevTools Memory tab for UI (heap snapshots, allocation timeline)
- Common issues: closure captures retaining large objects, event listeners not unregistered, circular references in React effects

**Priority:** P1

#### US-8.3.3 — Event Processing Performance
**As a** system
**I want** D-Bus events to be processed in under 1ms each
**so that** the system remains responsive under high event load.

**Acceptance Criteria:**
- D-Bus signal → event → IPC → UI render target: <5ms end-to-end
- Daemon event processing (signal → normalize → queue → emit): <1ms
- Multiple events in quick succession: processed in FIFO order with no backlog
- Backlog limit: max 1000 unprocessed events (drop oldest)
- No `std::sync::Mutex` contention in hot paths (use `tokio::sync`)

**Edge Cases:**
- 100 events in 1 second (e.g., rapid volume changes) → throttle to 10 per second
- Event storm (DoS from D-Bus) → rate limiter

**Technical Notes:**
- Use `tokio::sync::mpsc` channels with bounded capacity
- Event processing on a dedicated task per event type
- Metrics: emit event processing latency as a gauge for monitoring

**Priority:** P2

---

## Feature 8.4: Edge Case Handling

**Description:** Systematically identify and handle edge cases across all features: network disconnection, missing hardware, permission denied, daemon crashes, corrupt config, and concurrency issues.

**Acceptance Criteria:** All known edge cases documented and handled. Graceful degradation for missing features. Clear user feedback for error states. No silent failures.

**Dependencies:** All Epics

**Notes:** Create a edge case matrix covering every feature and its failure modes.

### User Stories

#### US-8.4.1 — Error State Inventory
**As a** developer
**I want** a comprehensive inventory of error states for every feature
**so that** none are left unhandled.

**Acceptance Criteria:**
- Error state matrix created and documented
- Each feature has documented: expected errors, error handling behavior, user feedback
- Error states covered:
  - D-Bus service not available (no NetworkManager, no BlueZ, no MPRIS player)
  - Permission denied
  - Hardware not present (no battery, no WiFi)
  - Network timeout/error
  - Corrupt config or cache
  - Daemon crash
  - IPC disconnect
  - WASM plugin crash
- Each error state has a defined response: log, notify user, fall back gracefully

**Edge Cases:**
- Transient errors (D-Bus service momentarily unavailable) → retry with backoff

**Technical Notes:**
- Document as part of architecture docs: `docs/error-handling.md`
- Use typed error enums in Rust for all error paths

**Priority:** P1

#### US-8.4.2 — Graceful Degradation
**As an** end user
**I want** the Dynamic Island to continue working even when some features fail
**so that** a single failure does not break the entire application.

**Acceptance Criteria:**
- Battery monitor fails → battery icon hidden, other features continue
- MPRIS listener fails → media features unavailable, pill shows placeholder
- Notification interception fails → native notifications continue working, island does not duplicate
- Plugin crashes → only that plugin is disabled, not all plugins
- D-Bus session bus stops → daemon goes into "degraded" state, UI shows disconnected indicator

**Edge Cases:**
- Multiple features fail → each degrades independently
- Feature recovers from failure → re-enable automatically (if transient)

**Technical Notes:**
- Feature health tracker: each feature has a health state (Healthy, Degraded, Failed)
- IPC includes health summary in status events
- UI shows feature-level error indicators if needed

**Priority:** P1

---

## Feature 8.5: Visual & Interaction Polish

**Description:** Final pass on visual polish: icon quality, typography, spacing, color harmony, and interaction feel. Ensure the application looks like it belongs on a modern Linux desktop.

**Acceptance Criteria:** All icons are crisp at all DPI levels. Typography uses system font stack. Spacing is consistent. Colors are harmonious. No visual glitches, flickers, or layout shifts.

**Dependencies:** 2.2, 2.5

**Notes:** This is a design review pass. Small visual improvements compound into a premium feel.

### User Stories

#### US-8.5.1 — Icon Quality
**As an** end user
**I want** all icons in the Dynamic Island to be sharp and consistent
**so that** the interface looks polished.

**Acceptance Criteria:**
- All icons are SVG (not raster)
- Icons use consistent stroke width (1.5px) and corner radius
- Icons are visually centered (optical alignment, not mathematical)
- Icons at 16×16px are legible (no overly complex icons at small sizes)
- Missing icons show a consistent fallback (colored circle with first letter)

**Edge Cases:**
- Fractional DPI scaling → SVG renders cleanly at any scale

**Technical Notes:**
- SVG icons as React components (or inline SVGs)
- Icon set should follow a consistent style guide (e.g., outlined, rounded corners)
- Consider using `lucide-react` or `heroicons` for the base icon set

**Priority:** P2

#### US-8.5.2 — Typography
**As an** end user
**I want** text in the Dynamic Island to be legible and well-proportioned
**so that** the interface is easy to read.

**Acceptance Criteria:**
- Font stack: system UI fonts (`-apple-system, 'Segoe UI', Roboto, 'Noto Sans', sans-serif`)
- Font sizes: pill text 13px, expanded title 16px, expanded body 14px, labels 12px
- Line height: 1.3 for single-line, 1.4 for multi-line
- Font weight: 500 (medium) for emphasis, 400 (normal) for body
- Letter spacing: default (no explicit tracking)
- No font loading (use system fonts only)

**Edge Cases:**
- Non-Latin characters (CJK, Arabic, etc.) → system font handles natively
- Very long words → `overflow-wrap: break-word`

**Technical Notes:**
- Define typography tokens in CSS custom properties
- Font sizes in `rem` for accessibility scaling

**Priority:** P2

#### US-8.5.3 — Visual Consistency Review
**As a** developer
**I want** to review the entire application for visual consistency
**so that** every element follows the same design language.

**Acceptance Criteria:**
- Review checklist:
  - All corners use 18px radius (pill) or 8px (inner elements)
  - All spacing uses multiples of 4px (4, 8, 12, 16, 20, 24)
  - All shadows/blurs use consistent values
  - All colors come from the theme palette
  - All transitions use the same easing curves
  - No hard-coded colors outside theme variables
- Fix any inconsistencies found

**Edge Cases:**
- Third-party UI (plugin contributions) → pass design guidelines to plugin authors

**Technical Notes:**
- Create a `tailwind.config.js` or CSS custom properties with all design tokens
- Review all components against the tokens

**Priority:** P2
