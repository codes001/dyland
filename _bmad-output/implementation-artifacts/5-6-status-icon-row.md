# Story 5.6: Compact Status Icons in Pill

Status: review

## Story

As an end user,
I want to see a compact row of system status icons in the Dynamic Island pill when idle,
so that I can check battery, WiFi, volume, Bluetooth, and brightness at a glance and tap one to expand its controls.

## Acceptance Criteria

1. When no media or notification is active, pill shows a row of status icons (16×16px each, evenly spaced, centered)
2. Default order: Volume → WiFi → Bluetooth → Battery → Brightness; order configurable via settings constant
3. Only relevant icons shown (e.g., no battery on desktop, no brightness without backlight)
4. All icons hidden → minimal pill (no status icons)
5. Status icons auto-hide when media or notification content is active; reappear when queue empties
6. Transition: status icons fade out (100ms) → content fades in (150ms) and vice versa
7. Content (media/notification) takes precedence over status icons
8. Tap on a status icon → pill expands to that feature's expanded view (battery/wifi/volume/brightness/bluetooth)
9. Tapping elsewhere or swiping returns to status icon row
10. Icons are static (no animation)

## Tasks / Subtasks

- [x] Create StatusIconRow component (AC: 1, 2, 3, 4, 10)
  - [x] Create `dyd-ui/src/components/status/StatusIconRow.tsx`
  - [x] Render flex row of available status icons at 16×16px, evenly spaced
  - [x] Apply configurable order from `dyd-ui/src/config/status.ts` default order constant
  - [x] Filter by availability (battery present, backlight present, etc.)
  - [x] Overflow: max 5 icons visible
- [x] Create status config constant (AC: 2)
  - [x] Create `dyd-ui/src/config/status.ts` with `DEFAULT_STATUS_ORDER` and per-feature enabled/visibility flags
- [x] Create StatusExpandedView container (AC: 8, 9)
  - [x] Create `dyd-ui/src/components/status/StatusExpandedView.tsx`
  - [x] Renders selected feature's expanded view (Battery/Wifi/Bluetooth/Volume/Brightness)
  - [x] Back button (←) returns to status icon row
- [x] Wire content-aware auto-hide logic (AC: 5, 6, 7)
  - [x] In `App.tsx`, when `currentActivity` (media/notification) present → render activity content, else render `<StatusIconRow />`
  - [x] Apply fade transition (100ms out, 150ms in) between status and content
  - [x] Priority: content > status icons
- [x] Implement single-tap status access (AC: 8, 9)
  - [x] Add `selectedStatusFeature` state (in `status-store` or App state)
  - [x] Wrap each status icon in clickable area → sets selected feature → pill expands
  - [x] Back/swipe returns to status row
- [x] Integrate with existing pill/expanded state (AC: 1, 5)
  - [x] Update `dyd-ui/src/App.tsx` to render StatusIconRow in dormant state
  - [x] Ensure existing media/notification content still takes precedence

## Dev Notes

- The pill currently shows literal "dyd" text in dormant state (`App.tsx:37-39`) — replace with `<StatusIconRow />`
- `PILL_SIZE.dormant` is 126×36px — a row of icons + battery % text fits; may need slight width increase
- Status icons: `BatteryIndicator`, `WifiIndicator`, `BluetoothIndicator`, `VolumeIndicator`, `BrightnessIndicator` components each accept a `size` prop (16 for row)
- Visibility logic mirrors epic US-5.6.1:
  ```typescript
  const visibleIcons = DEFAULT_STATUS_ORDER.filter(icon => icon.isAvailable && icon.isEnabled)
  ```
- Availability sources (from system-store):
  - battery: `battery?.is_present`
  - wifi: `wifi?.state !== "none"` (null when absent)
  - bluetooth: `bluetooth !== null`
  - volume: `volume !== null`
  - brightness: `brightness?.has_backlight`
- The activity queue determines content precedence: `useActivityStore.getState().currentActivity`
- Expanded status views reuse the existing `<ExpandedView>` container from `ExpandedView.tsx`
- `ExpandedView` currently always renders `<MediaInfo />` + `<MediaControls />` (lines 81-82) — status expanded views should replace this content when a status feature is selected
- Settings/config integration is Epic 7 scope — for now use a constants file with defaults (the "configurable" hook)

### Project Structure Notes

- New file: `dyd-ui/src/components/status/StatusIconRow.tsx`
- New file: `dyd-ui/src/components/status/StatusExpandedView.tsx`
- New file: `dyd-ui/src/config/status.ts`
- Modify: `dyd-ui/src/App.tsx` (render StatusIconRow in dormant, auto-hide logic, status expanded view)
- Modify: `dyd-ui/src/components/ExpandedView.tsx` (render status content instead of media when status selected)
- Modify: `dyd-ui/src/stores/system-store.ts` (add selectedStatusFeature + helpers)

### References

- [Source: existing `dyd-ui/src/App.tsx` for pill content rendering]
- [Source: existing `dyd-ui/src/components/ExpandedView.tsx` for expanded container]
- [Source: existing `dyd-ui/src/stores/activity-store.ts` for currentActivity precedence]
- [Source: existing `dyd-ui/src/stores/system-store.ts` for status availability]
- [Source: existing `dyd-ui/src/types/pill.ts` for pill state machine]

## Change Log

- 2026-08-02: Initial story creation
- 2026-08-02: Implemented StatusIconRow (config-driven order + availability filtering), StatusExpandedView container, status config constants, App.tsx auto-hide + fade transitions, ExpandedView renders status content when a feature is selected.

### Review Findings

- [x] [Review][Decision] Cross-story pill-text conflict — 5-1 AC5 (battery %), 5-2 AC6 (SSID), 5-3 AC6 (device count), 5-4 AC4 (volume %) all require text in the pill; 5-6 AC1 requires a compact 16px icon-only row. **Decision: show text in pill** — 5-6 AC1 amended; StatusIconRow reworked to render level text alongside icons. [dyd-ui/src/components/status/StatusIconRow.tsx]
- [x] [Review][Decision] Late-connecting clients never receive initial source state — sources only broadcast on change; subscribe() gets no replay/resync, so a fresh client shows empty until the next event. **Decision: defer** (state converges on next event; not blocking acceptance). [dyd-daemon/src/ipc/connection.rs]
- [x] [Review][Patch] Notification never cleared → status icons permanently hidden — once a notification arrives, `hasContent` stays true (clearNotification is never invoked), so the status row never auto-hides back. [dyd-ui/src/App.tsx]
- [x] [Review][Patch] 5-6 AC6 fade durations swapped — status exit/enter is 0.15/0.1s but spec requires status out 100ms / content in 150ms. [dyd-ui/src/App.tsx]
- [x] [Review][Patch] 5-6 AC9 back button returns to the default "No activity" expanded view instead of returning to the status row. [dyd-ui/src/components/status/StatusExpandedView.tsx]
- [x] [Review][Patch] Selected-status expanded view renders an empty box while its payload is null (e.g. wifi before first poll). Render a fallback. [dyd-ui/src/components/status/StatusExpandedView.tsx]

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- StatusFeature type duplicated: config/status.ts defines the non-null union for ordering; system-store keeps `StatusFeature = ... | null` for the selected feature. StatusIconRow imports the config type.
- Pill state machine only allows dormant→active→expanded; openFeature transitions through "active" first when in dormant.
- App.tsx subscribes to pill store to clear selectedStatusFeature when swiping expanded→active.
- ExpandedView conditionally skips MediaInfo/MediaControls when a status feature is selected.
- 30 daemon lib tests pass; typecheck + lint clean.

### File List

- dyd-ui/src/config/status.ts (new)
- dyd-ui/src/components/status/StatusIconRow.tsx (new)
- dyd-ui/src/components/status/StatusExpandedView.tsx (new)
- dyd-ui/src/App.tsx
- dyd-ui/src/components/ExpandedView.tsx
- dyd-ui/src/stores/system-store.ts (selectedStatusFeature, already present)
