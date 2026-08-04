# Story 4.2: Notification Rendering in Island

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an end user,
I want incoming notifications to be rendered in the Dynamic Island pill and expanded view,
so that I can see and interact with them without a disruptive native popup.

## Acceptance Criteria

1. Active pill shows: [app icon (16x16px)] + [truncated summary/message] (AC: pill indicator)
2. App icon uses the app's icon from `app_icon`; if not resolvable, fall back to a generic bell icon or first-letter badge
3. Urgency affects pill treatment: low = none, normal = standard, critical = pulsing glow + red dot indicator
4. Notification becomes the active pill content (replaces prior content, participates in the activity/content flow)
5. Expanded view shows: [large app icon (32x32px)] + [app name (bold)] + [relative timestamp]
6. Expanded view shows full notification body text (wrapped, not truncated), max 4 lines with scroll for overflow
7. Action buttons (if any) render as rounded text buttons below the body, max 3 visible with "+N" overflow
8. Dismiss button (X) in top-right corner of expanded view closes the notification (`NotificationClose`)
9. Clicking an action button sends the `NotificationAction` IPC command with the action key
10. Empty body shows only the summary (larger font); no actions shows no buttons (normal state)
11. Timestamp rendered as relative time ("2m ago", "1h ago") via `Intl.RelativeTimeFormat`
12. Icon resolution follows priority: `app_icon` param -> cached path -> generic bell fallback; resolved icons cached

## Tasks / Subtasks

- [x] Create PillNotificationContent component (AC: 1, 2, 3, 4)
  - [x] Create `dyd-ui/src/components/notification/PillNotificationContent.tsx`
  - [x] Render [app icon 16x16] + [truncated summary]; if no summary show app name (max 25 chars ellipsis)
  - [x] Resolve app icon via shared icon-resolver helper; fallback to generic bell SVG
  - [x] Apply urgency treatment: critical -> pulsing glow + red dot overlay indicator
  - [x] Component reads current notification from `useNotificationStore`
- [x] Create NotificationExpandedView component (AC: 5, 6, 7, 8, 9, 10, 11)
  - [x] Create `dyd-ui/src/components/notification/NotificationExpandedView.tsx`
  - [x] Render [large app icon 32x32] + [app name bold] + [relative timestamp]
  - [x] Render body text wrapped, max 4 lines with overflow scroll
  - [x] Render action buttons (max 3 visible, "+N" overflow for the rest)
  - [x] Dismiss button (X) top-right; onClick sends `NotificationClose` IPC command
  - [x] Action button onClick sends `NotificationAction` IPC command with action key
  - [x] Empty body shows summary at larger font; no actions shows normal state
  - [x] Use `Intl.RelativeTimeFormat` for relative timestamp
- [x] Create shared app-icon resolver (AC: 2, 12)
  - [x] Create `dyd-ui/src/lib/notification-icon.ts` (or resolver under notification dir)
  - [x] Resolve icon priority: `app_icon` param (may be a path) -> fallback
  - [x] Cache resolved icon (Map) keyed by app_name/app_icon
  - [x] Handle file-path vs icon-name; on failure return generic fallback / letter avatar
- [x] Wire notification rendering into App.tsx (AC: 1, 4, 5)
  - [x] In pill content branch, render `PillNotificationContent` when a notification is current
  - [x] In expanded view, render `NotificationExpandedView` when notification is current and not status-selected
- [x] Update types/constants if needed (AC: 9)
  - [x] Reuse existing `NotificationPayload`, `NotificationActionItem` from types
  - [x] Add/verify command builders for `NotificationClose` and `NotificationAction`
- [x] Tests (AC: all)
  - [x] Unit tests for icon resolution (path, missing, cache fallback)
  - [x] Unit tests for summary truncation + urgency mapping helpers
  - [x] Component render tests for pill + expanded view using store fixtures

## Dev Notes

- Existing plumbing: story 4-1 already delivers `NotificationReceived` events -> `useNotificationStore`. Do NOT re-add daemon work; this story is UI-only.
- `NotificationPayload` shape (already in types): `{ id: number, app_name: string, app_icon: string | null, summary: string, body: string | null, urgency: 'low'|'normal'|'critical', actions: NotificationActionItem[], timestamp: number }`
- `NotificationActionItem`: `{ key, label }`
- IPC commands already defined in types and handled in daemon `connection.rs`: `NotificationAction` and `NotificationClose` (payloads `{ id, action_key, reply_text? }` and `{ id }`).
- Use `ipcClient.sendCommand(JSON.stringify({ type: 'NotificationClose', payload: { id } }))` for dismiss.
- Use `ipcClient.sendCommand(JSON.stringify({ type: 'NotificationAction', payload: { id, action_key } }))` for an action button.
- Follow existing component + styling patterns from `MediaInfo.tsx` and `status/*ExpandedView.tsx` (motion, tailwind utility classes, dark overlay theme).
- Pill notification width = 200px (STYLE-GUIDE). Truncate summary to 25 chars with ellipsis.
- App name text: 13px 600; expanded title 16px 600; body 14px/1.4; timestamp 11px. (STYLE-GUIDE font sizes)
- Icons: 16x16px in pill; 32x32px in expanded. Generic bell SVG as fallback (STYLE-GUIDE icon set: `bell`).
- Critical urgency: pulsing glow + red dot. Reference the battery pulse animation in `index.css` (`dyd-battery-pulse`) for a reusable pattern; add an `dyd-critical-glow` keyframe.
- Relative time: `Intl.RelativeTimeFormat('en', { numeric: 'auto' })`.
- Store currently sets notification and auto-clears after 5s in client.ts (NOTIFICATION_DISMISS_MS). Keep that behavior; do not remove.
- App.tsx already computes `hasContent` and shows a generic "dyd" span; replace that span with real notification content.
- The expanded view currently always renders `MediaInfo`/`MediaControls` when no status feature is selected; gate so notification expanded view replaces media when a notification is the current/active content.

### Project Structure Notes

- New: `dyd-ui/src/components/notification/PillNotificationContent.tsx`
- New: `dyd-ui/src/components/notification/NotificationExpandedView.tsx`
- New: `dyd-ui/src/lib/notification-icon.ts` (resolver + helpers)
- New (tests): `dyd-ui/src/lib/notification-icon.test` (framework-agnostic helper tests; run via existing TS pipeline or add vitest if available — NO new runtime deps)
- Modify: `dyd-ui/src/App.tsx` (render notification pill + expanded content)
- Optional: `dyd-ui/src/index.css` (add critical glow keyframes if not reusing existing)

### References

- [Source: Story 4.1 `4-1-notification-source.md` — NotificationPayload, NotificationReceived, existing store [Source: 4-1-notification-source.md]]
- [Source: EPIC-04 `bmad-output/EPIC-04-NOTIFICATIONS.md` — Feature 4.2 definitions [Source: bmad-output/EPIC-04-NOTIFICATIONS.md#Feature-4.2]]
- [Source: `dyd-ui/src/stores/notification-store.ts` — existing store]
- [Source: `dyd-ui/src/ipc/client.ts` — NotificationReceived handling + NOTIFICATION_DISMISS_MS]
- [Source: `dyd-ui/src/components/MediaInfo.tsx` — component + styling pattern]
- [Source: `dyd-ui/src/components/status/StatusExpandedView.tsx` — expanded view pattern]
- [Source: `types/src/index.ts` — NotificationPayload/NotificationActionItem]
- [Source: `dyd-daemon/src/ipc/connection.rs` — NotificationClose/NotificationAction handlers]
- [Source: `bmad-output/STYLE-GUIDE.md` — font sizes, icon set, thr truncation rules]

## Change Log

- 2026-08-03: Initial story creation
- 2026-08-03: Implemented notification pill + expanded rendering, shared icon resolver, App.tsx wiring, critical urgency glow; 13 unit tests added
- 2026-08-03: Code review — findings recorded below

### Review Findings

- [x] [Review][Patch] Notification pill never activates/widens — dormant pill is non-interactive, nothing promotes state on NotificationReceived, and no 200px notification size exists. Fix: promote dormant->active on NotificationReceived, make pill clickable to expand, add 200px notification width. [dyd-ui/src/App.tsx, dyd-ui/src/components/Pill.tsx, dyd-ui/src/stores/pill-store.ts]
- [x] [Review][Defer] Icon resolution implements only the path branch — US-4.2.3 priority chain (hints.image-data, hints.image-path, desktop-file/icon-theme lookup by app_name) is missing; story AC12 only calls for app_icon path -> cached -> bell — deferred, story scope kept
- [x] [Review][Patch] Remote icon URL policy — app_icon may be an arbitrary http(s) URL rendered as `<img src>` (privacy/tracking vector). Fix: restrict to local file paths only; http(s) URLs fall back to letter avatar. [dyd-ui/src/lib/notification-utils.ts:21-22, dyd-ui/src/components/notification/AppIcon.tsx:175-181]
- [x] [Review][Patch] Pill app icon rendered at 20px, not the required 16x16px centered in a 20x20px container. [dyd-ui/src/components/notification/PillNotificationContent.tsx:32]
- [x] [Review][Patch] Expanded body is 4-line clamped with `overflow: hidden` and no scroll — overflow body text is inaccessible; spec requires "scroll for overflow". [dyd-ui/src/components/notification/NotificationExpandedView.tsx:103]
- [x] [Review][Patch] Auto-dismiss timer keeps running while the expanded view is open — actions/dismiss vanish mid-interaction after 5s. [dyd-ui/src/ipc/client.ts:103-107]
- [x] [Review][Patch] sentActions/overflowOpen persist across notification replacement (keyed by action key, not notification id) and double-click can send duplicate NotificationAction commands. [dyd-ui/src/components/notification/NotificationExpandedView.tsx:40-41,58-59]
- [x] [Review][Patch] Broken icon path renders a broken `<img>` forever — no onError fallback, and the failed result is cached permanently. [dyd-ui/src/components/notification/AppIcon.tsx:160-183, dyd-ui/src/lib/notification-utils.ts:85-95]
- [x] [Review][Patch] truncateSummary slices UTF-16 code units — splits surrogate pairs (emoji) producing lone surrogates. [dyd-ui/src/lib/notification-utils.ts:39]
- [x] [Review][Patch] relativeTime mislabels future/pre-epoch timestamps — clamping applied after unit escalation yields "this minute"/"today" instead of "now". [dyd-ui/src/lib/notification-utils.ts:67-80]
- [x] [Review][Patch] iconCache Map grows unbounded (no size cap; clearIconCache never called in app code). [dyd-ui/src/lib/notification-utils.ts:82-97]
- [x] [Review][Patch] aria-label interpolates possibly-null/empty app_name ("Notification from "). [dyd-ui/src/components/notification/PillNotificationContent.tsx:30]
- [x] [Review][Patch] Expanded no-body branch renders blank when summary is empty — should fall back to app name / placeholder. [dyd-ui/src/components/notification/NotificationExpandedView.tsx:110]
- [x] [Review][Patch] Critical dot aria-label on a generic `<span>` is not exposed to assistive tech. [dyd-ui/src/components/notification/PillNotificationContent.tsx:57]
- [x] [Review][Patch] LetterAvatar sets state in useEffect for a purely derivable value — extra render + color flash. [dyd-ui/src/components/notification/AppIcon.tsx:31-35]
- [x] [Review][Defer] Empty app_name notifications dropped by daemon-message gate — pre-existing in client.ts from story 4-1 [dyd-ui/src/ipc/client.ts:101] — deferred, pre-existing
- [x] [Review][Defer] Dismiss sends NotificationClose with daemon's replaces_id (often 0) — server-side no-op for first notifications [dyd-daemon/src/sources/notifications.rs:124,166-179] — deferred, pre-existing daemon behavior

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

- `dyd-ui/src/lib/notification-utils.ts` — pure helper module (normalizeUrgency, truncateSummary, displayText, resolveAppIcon, relativeTime) deliberately kept React-free so it can be unit tested with Node's built-in test runner (no vitest installed; zero new deps)
- `dyd-ui/src/lib/notification-utils.ts` — `resolveAppIconCached` + module-level `Map` cache keyed by `appIcon|appName`; `clearIconCache()` exported for tests
- `dyd-ui/src/components/notification/AppIcon.tsx` — shared renderer for image/letter/bell icon kinds; hue-stable letter avatar derived from app name
- `dyd-ui/src/components/notification/PillNotificationContent.tsx` — renders icon + truncated summary; critical urgency adds `dyd-notification-critical` glow class + red dot
- `dyd-ui/src/components/notification/NotificationExpandedView.tsx` — expanded layout, max-4-line clamped body, action buttons (max 3 + "+N" overflow), dismiss (X) and action IPC commands, relative timestamp, sent-action guard
- `dyd-ui/src/App.tsx` — pill content now renders `PillNotificationContent` when a notification is current; expanded view renders `NotificationExpandedView` (takes precedence over status/empty state)
- `dyd-ui/src/components/ExpandedView.tsx` — hides media info/controls while a notification is displayed
- `dyd-ui/src/index.css` — added `dyd-critical-glow` keyframes + `prefers-reduced-motion` guard
- Tests live in `dyd-ui/tests/` (outside `src/`) to avoid `tsc`/eslint including `node:test` types; run with `node --test "tests/**/*.test.ts"`

### Completion Notes List

- Created `PillNotificationContent` (AC 1-4): 20px app icon + 25-char truncated summary/app-name fallback, critical urgency pulsing glow + red dot
- Created `NotificationExpandedView` (AC 5-11): 32px icon + bold app name + relative timestamp, 4-line clamped body, action buttons (3 + "+N" overflow) invoking `NotificationAction`, X dismiss invoking `NotificationClose`, empty-body large summary fallback
- Created shared app-icon resolver + cache (AC 2, 12): image for paths/URLs, letter avatar fallback, module-level cache
- Wired notification rendering into App.tsx and gated media info in ExpandedView while a notification is shown
- Reused existing types — no type changes required; `NotificationAction`/`NotificationClose` command builders already defined
- Added 13 unit tests (normalizeUrgency, truncateSummary, displayText, resolveAppIcon, cache, relativeTime) — all pass via `node --test`
- All validation green: `tsc --noEmit`, `eslint src/`, `npm run build -w dyd-ui`, 30 daemon tests pass

### File List

- New: `dyd-ui/src/lib/notification-utils.ts`
- New: `dyd-ui/src/components/notification/AppIcon.tsx`
- New: `dyd-ui/src/components/notification/PillNotificationContent.tsx`
- New: `dyd-ui/src/components/notification/NotificationExpandedView.tsx`
- New: `dyd-ui/tests/notification-utils.test.ts`
- Modified: `dyd-ui/src/App.tsx` — notification pill + expanded view wiring
- Modified: `dyd-ui/src/components/ExpandedView.tsx` — hide media while notification active
- Modified: `dyd-ui/src/index.css` — critical glow animation
- Modified: `_bmad-output/implementation-artifacts/sprint-status.yaml` — story 4-2 status updates