# Story 3.2: Media Info Display (UI-side)

Status: review

## Story

As a user,
I want to see the currently playing media (title, artist, album) displayed in the Dynamic Island,
so that I can glance at what's playing without switching applications.

## Acceptance Criteria

1. A Zustand media store (`media-store.ts`) holds current media state: player name, title, artist, album, art_url, position, playback_status
2. `client.ts` handles the `MediaChanged` IPC event and updates the media store
3. A `MediaInfo` component renders now‑playing info: title (bold), artist, album art placeholder
4. The component is shown inside the expanded Pill when media is playing
5. When playback_status is "stopped" or no player is active, the media section is hidden
6. The component animates in/out using motion.div with a fade+slide transition (200ms)
7. Title text scrolls (marquee) if it overflows the pill width
8. Position/progress bar shows current playback position relative to total length

## Tasks / Subtasks

- [x] Create media Zustand store (AC: 1)
  - [x] Create `dyd-ui/src/stores/media-store.ts`
  - [x] Define `MediaState`: player, title, artist, album, artUrl, length, position, playbackStatus
  - [x] Implement `setMedia(payload)` action that updates all fields
  - [x] Implement `clearMedia()` action for stopped state
  - [x] Add `isPlaying` computed getter (playback_status === "playing")
- [x] Handle MediaChanged in IPC client (AC: 2)
  - [x] Add `case "MediaChanged"` to the switch statement in `client.ts`
  - [x] Dispatch payload to `useMediaStore.getState().setMedia()`
  - [x] On stopped/empty → dispatch `clearMedia()`
- [x] Create MediaInfo component (AC: 3, 4, 5, 6, 7)
  - [x] Create `dyd-ui/src/components/MediaInfo.tsx`
  - [x] Render title (bold, truncate with marquee on overflow), artist, album
  - [x] Wrap in `motion.div` with fade+slide animation (200ms)
  - [x] Conditionally render — null if no active media or status === "stopped"
  - [x] Add CSS for marquee/scrolling text on overflow
- [x] Add progress bar (AC: 8)
  - [x] Render a thin progress bar below the media info text
  - [x] Width = (position / length) * 100%
  - [x] Animate progress bar smoothly using motion.div layout animations
  - [x] Handle length=0 gracefully (no progress shown)
- [x] Integrate into Pill/ExpandedView (AC: 4, 5)
  - [x] Import MediaInfo in `ExpandedView.tsx`
  - [x] Render `<MediaInfo />` inside the expanded content area
  - [x] Auto-expand pill when media starts playing

## Dev Notes

- Media store follows same pattern as `activity-store.ts` (Zustand with create)
- The `motion` library is already installed (v12.42.2)
- `client.ts` already imports `usePillStore` and `useActivityStore` — follow the same pattern
- The MediaInfo component should be self-contained (no side effects)
- Title marquee: use CSS `@keyframes marquee` + `overflow: hidden` + `text-overflow: ellipsis` fallback
- The ExpandedView already uses `motion.div` for backdrop blur
- Follow existing patterns: `Pill.tsx` uses `AnimatePresence` for content transitions

### Project Structure Notes

- New files: `dyd-ui/src/stores/media-store.ts`, `dyd-ui/src/components/MediaInfo.tsx`
- Modified: `dyd-ui/src/ipc/client.ts`, `dyd-ui/src/components/ExpandedView.tsx`

### References

- [Source: existing `dyd-ui/src/stores/activity-store.ts` for Zustand store pattern]
- [Source: existing `dyd-ui/src/ipc/client.ts` for IPC event handling pattern]
- [Source: existing `dyd-ui/src/components/ExpandedView.tsx` for animation patterns]

## Change Log

- 2026-07-19: Initial story creation
- 2026-07-19: Full implementation — media store, MediaInfo component, IPC wiring

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Created `dyd-ui/src/stores/media-store.ts` with Zustand store for media state
- Created `dyd-ui/src/components/MediaInfo.tsx` with MarqueeText, ArtImage, ProgressBar
- Updated `client.ts` to handle MediaChanged events and dispatch to media store
- Updated `ExpandedView.tsx` to render MediaInfo and auto-expand on media play
- Added marquee CSS animation in index.css

### File List

- dyd-ui/src/stores/media-store.ts (new)
- dyd-ui/src/components/MediaInfo.tsx (new)
- dyd-ui/src/ipc/client.ts (modified)
- dyd-ui/src/components/ExpandedView.tsx (modified)
- dyd-ui/src/index.css (modified)
