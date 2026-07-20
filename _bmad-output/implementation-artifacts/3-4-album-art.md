# Story 3.4: Album Art Display

Status: review

## Story

As a user,
I want to see album artwork in the Dynamic Island when media is playing,
so that I can visually identify the currently playing content.

## Acceptance Criteria

1. Album art is fetched from the `mpris:artUrl` property (file:// or https:// URL)
2. Art is displayed as a rounded square thumbnail in the expanded Pill (48x48px or 64x64px)
3. Art is positioned to the left of the media info text
4. Fallback: if no artUrl, show a music note icon placeholder (SVG)
5. Art fades in on load (CSS transition opacity 0→1, 300ms)
6. Loading state: subtle shimmer/pulse placeholder while art loads
7. Error state: broken image fallback (music note icon)
8. Caching: art URLs are cached in memory to avoid re-fetching during the same session

## Tasks / Subtasks

- [x] Add album art to MediaInfo component (AC: 1, 2, 3)
  - [x] In `MediaInfo.tsx`, add `<img>` element for album art
  - [x] Position art to the left of title/artist using flexbox layout
  - [x] Set fixed dimensions: 48x48px with `rounded-lg` (Tailwind)
  - [x] Use `artUrl` from media store as src attribute
- [x] Implement image loading lifecycle (AC: 5, 6, 7)
  - [x] Add `onLoad` handler: set opacity to 1 with CSS transition
  - [x] Add `onError` handler: hide img, show fallback icon
  - [x] Show shimmer/skeleton placeholder while loading (CSS `animate-pulse`)
  - [x] Use `useState` for loading/error state management
- [x] Implement fallback icon (AC: 4, 7)
  - [x] Create inline SVG music note icon for fallback
  - [x] Same dimensions as album art (48x48px)
  - [x] Muted color (text-white/40)
- [x] Implement in-memory caching (AC: 8)
  - [x] Add `artCache: Map<string, string>` to media store
  - [x] Before fetching, check cache by artUrl
  - [x] Clear cache when media is cleared
- [x] Handle file:// protocol (AC: 1)
  - [x] Check if `convertFileSrc` from `@tauri-apps/api/core` is available
  - [x] If artUrl is `file:///...`, convert to Tauri asset URL
  - [x] If not running in Tauri (browser dev), use placeholder for file:// URLs

## Dev Notes

- Album art in MPRIS comes from `mpris:artUrl` in the Metadata property
- Common formats: `file:///home/user/.cache/spotify/art/xxx`, `https://i.scdn.co/image/...`
- For file:// URLs in Tauri, use `convertFileSrc()` from `@tauri-apps/api/core`
- Tailwind CSS classes: `w-12 h-12 rounded-lg object-cover`, `animate-pulse` for shimmer
- The music note SVG icon can be a simple inline SVG path
- For browser dev mode (npm run dev), file:// URLs won't work — show fallback
- In-memory cache avoids repeated Tauri file reads

### Project Structure Notes

- Modified only: `dyd-ui/src/components/MediaInfo.tsx`, `dyd-ui/src/stores/media-store.ts`

### References

- [Tauri convertFileSrc docs](https://v2.tauri.app/reference/javascript/api/core/#convertfilesrc)
- [Source: existing `dyd-ui/src/components/MediaInfo.tsx` from Story 3.2]

## Change Log

- 2026-07-19: Initial story creation
- 2026-07-19: Full implementation — album art in MediaInfo, caching, fallback

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Album art implemented within MediaInfo.tsx: ArtImage component with loading/error states
- Fallback music note SVG icon for missing/broken art
- Shimmer pulse animation while loading
- In-memory art cache in media-store.ts (Map<string, string>)
- Handles file:// and https:// URLs — falls back to icon for unsupported protocols

### File List

- dyd-ui/src/components/MediaInfo.tsx (modified)
- dyd-ui/src/stores/media-store.ts (modified)
