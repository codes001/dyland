# Epic 3: Media Integration

## Objective
Implement MPRIS (Media Player Remote Interfacing Specification) integration to display currently playing media in the Dynamic Island with playback controls, album art, and multi-source support.

## Business Value
Media controls are one of the most frequently used Dynamic Island features on iPhone. This Epic delivers instant user value — every user plays music, podcasts, or video.

## Success Criteria
- Currently playing media appears in the pill (icon + truncated title)
- Expanded view shows: title, artist, album art, playback progress bar, controls
- Play/Pause, Next, Previous, and Seek controls work
- Album art renders with proper aspect ratio and transitions
- Multiple media sources (Spotify, Firefox, Rhythmbox, etc.) are supported
- Media state changes reflect in island within 200ms

## Risks
- MPRIS v2 spec is inconsistently implemented across media players
- Some browsers (Chromium) implement MPRIS, others (Firefox) need `playerctl` or custom D-Bus monitoring
- Album art may be large; need to cache and downscale to avoid memory issues
- Some media players do not expose seek capabilities

## Dependencies
- Epic 1 (IPC, daemon)
- Epic 2 (pill rendering, state machine, expanded view)

## Complexity
⭐⭐⭐ — Moderate. Standard protocol well-documented, but implementation variance across players requires defensive coding.

---

## Feature 3.1: MPRIS Event Listener

**Description:** Implement a D-Bus listener in the daemon that monitors MPRIS v2 media player signals: `PropertiesChanged` for track metadata, playback status, and player identity. Support multiple simultaneous players.

**Acceptance Criteria:** Daemon detects all running MPRIS-compatible media players. Listens for `PropertiesChanged` signals. Captures: `Metadata` (title, artist, album, art URL), `PlaybackStatus` (Playing/Paused/Stopped), `Identity` (player name). Handles player disconnect/reconnect.

**Dependencies:** 1.2 (daemon D-Bus connection)

**Notes:** Use `zbus` signal streaming. MPRIS players register on `org.mpris.MediaPlayer2.*` bus names.

### User Stories

#### US-3.1.1 — MPRIS Player Discovery
**As a** system
**I want** to discover all active MPRIS media players on the D-Bus session bus
**so that** I can subscribe to their media change events.

**Acceptance Criteria:**
- On startup, enumerate all names matching `org.mpris.MediaPlayer2.*`
- Subscribe to `NameOwnerChanged` on `org.freedesktop.DBus` to detect new/removed players
- Detected players are stored with: bus name, identity string, supported capabilities
- When a player disconnects, clean up and remove from active list
- Supported capabilities are detected by querying `org.mpris.MediaPlayer2.Player` interface

**Edge Cases:**
- Player starts after daemon → NameOwnerChanged signal arrives → subscribe
- Player crashes → NameOwnerChanged with empty owner → cleanup
- Multiple instances of same player (e.g., multiple Firefox windows) → track separately
- Player with broken MPRIS implementation → skip with warning

**Technical Notes:**
- Use `zbus::Connection::call_method` on `org.freedesktop.DBus` / `ListNames`
- Filter names matching regex `^org\.mpris\.MediaPlayer2\.`
- For each player, get identity: call `org.mpris.MediaPlayer2.Identity` property
- Get supported capabilities: check `CanPlay`, `CanPause`, `CanGoNext`, `CanGoPrevious`, `CanSeek` properties
- Store in `HashMap<String, MediaPlayer>` keyed by bus name

**Priority:** P0

#### US-3.1.2 — Metadata & Playback Status Subscription
**As a** system
**I want** to receive real-time updates when track metadata or playback status changes
**so that** the Dynamic Island can reflect current media state.

**Acceptance Criteria:**
- Subscribe to `org.freedesktop.DBus.Properties.PropertiesChanged` on each player's `org.mpris.MediaPlayer2.Player` interface
- Captured fields: `Metadata` (xesam:title, xesam:artist, xesam:album, mpris:artUrl), `PlaybackStatus`, `Position`
- Changes are emitted to the UI via IPC within 100ms of D-Bus signal receipt
- Signal handler is efficient (no blocking calls in the handler)

**Edge Cases:**
- Rapid metadata changes (e.g., streaming ads every 30s) → throttle to max 1 event per second
- Missing metadata fields (e.g., no album) → gracefully handle with empty strings
- PlaybackStatus transitions: Playing → Paused → Stopped → Playing

**Technical Notes:**
- Use `zbus::Connection::proxy` with `PropertyIterator` or streaming signal handler
- D-Bus signal handler runs on a dedicated `tokio::spawn` task
- Parse `Metadata` dictionary variant (dict of `String → Variant`)
- Convert to flat `MediaMetadata` struct:
  ```rust
  struct MediaMetadata {
      title: Option<String>,
      artist: Option<Vec<String>>,
      album: Option<String>,
      art_url: Option<String>,
      length: Option<i64>, // microseconds
  }
  ```

**Priority:** P0

#### US-3.1.3 — Position Tracking
**As an** end user
**I want** the media playback position to update in the Dynamic Island
**so that** I can see progress within the current track.

**Acceptance Criteria:**
- Initial position captured from `Position` property when track starts
- Position updates via `Seeked` signal (jump to new position)
- Do NOT poll position (use only signal-based updates for performance)
- Position is stored as microseconds and converted to display format (mm:ss)
- Progress bar in expanded view reflects current position

**Edge Cases:**
- `Seeked` signal may not be emitted by all players → fall back to estimated position based on elapsed time since last known position
- Position jumps (seeking) → `Seeked` signal provides new absolute position
- Track end → position resets to 0

**Technical Notes:**
- MPRIS `Position` is in microseconds
- Track length from `mpris:length` in metadata (also microseconds)
- Local position estimation: `current_position + elapsed_since_last_signal`
- Emit position updates to UI every 1 second only when position is changing (debounced)

**Priority:** P2

#### US-3.1.4 — Player-Specific Compatibility Shims
**As a** system
**I want** to handle MPRIS implementation differences across media players
**so that** the integration works reliably with all popular Linux media apps.

**Acceptance Criteria:**
- Known MPRIS implementations and their quirks:
  - Spotify: fully compliant MPRIS v2, art URL requires auth token → fetch with auth header
  - Firefox/Chromium: mostly compliant, art URL may be data: URI
  - Rhythmbox: compliant, some properties may be missing
  - VLC: compliant
  - mpv + mpris plugin: compliant
- Player-specific workarounds are documented and isolated
- Graceful degradation: if a player doesn't support a feature, hide that control

**Edge Cases:**
- Data URIs for album art → render inline (small images, base64 decode)
- Art URL requires HTTP authentication → fail silently, show placeholder
- Player reports `CanSeek: false` → hide seek bar

**Technical Notes:**
- Store a `PlayerCapabilities` struct per player:
  ```rust
  struct PlayerCapabilities {
      can_play: bool,
      can_pause: bool,
      can_go_next: bool,
      can_go_previous: bool,
      can_seek: bool,
  }
  ```
- Player identity matched against known patterns: `spotify`, `firefox`, `chromium`, etc.
- Workarounds implemented as trait methods on a `MprisCompat` trait

**Priority:** P1

---

## Feature 3.2: Now-Playing UI

**Description:** Render the currently playing media in the Dynamic Island pill and expanded view. Pill shows icon + truncated title. Expanded view shows full metadata, album art (if available), and progress bar.

**Acceptance Criteria:** Active pill shows music note icon + scrolling/truncated title. Expanded view shows art, title, artist, album, progress. Layout matches Apple's Dynamic Island media player aesthetic.

**Dependencies:** 2.1, 2.2, 3.1

**Notes:** Text that is too long for the pill should scroll (marquee) or truncate with ellipsis. Apple's island animates text sliding.

### User Stories

#### US-3.2.1 — Pill Now-Playing Indicator
**As an** end user
**I want** the pill to show a small music note icon and the song title when media is playing
**so that** I can identify the current track at a glance.

**Acceptance Criteria:**
- Active/expanded pill shows: [music icon] + [truncated title "Song Name…"]
- Max text length: ~20 characters before truncation (pill width minus icon)
- Icon is a simple SVG music note (white on black)
- Text color: white, font size: 13px, font: system UI
- When media is paused: icon changes to pause icon, text opacity reduces to 60%

**Edge Cases:**
- Very short title (< 5 chars) → no truncation needed
- No title in metadata → show "Unknown Track"
- Multiple artists → show "Artist1, Artist2" (truncated if needed)

**Technical Notes:**
- Component: `<PillMediaContent>` rendered inside `<Pill>` when media state is active
- Truncation: CSS `text-overflow: ellipsis; white-space: nowrap; overflow: hidden; max-width: 120px;`
- SVG icons as React components (inline SVGs for performance)
- Paused state: CSS `opacity: 0.6` on text + switch to pause icon

**Priority:** P0

#### US-3.2.2 — Expanded Now-Playing View
**As an** end user
**I want** to see full media details when I expand the Dynamic Island
**so that** I can view album art, complete title, artist, and progress.

**Acceptance Criteria:**
- Expanded view layout (from top to bottom):
  - Album art (rounded, 60×60px, positioned left or centered)
  - Track title (full, not truncated, 16px bold)
  - Artist name (14px, secondary color)
  - Album name (if available, 12px, tertiary color)
  - Playback progress bar (full width)
  - Controls row (play/pause, prev, next)
- Layout is clean, minimal, and matches Apple's design language
- Dark background with backdrop blur

**Edge Cases:**
- No album art → show a stylized music note placeholder (gradient background)
- Very long title → wrap to 2 lines max, truncate with ellipsis
- Very long artist list → truncate with "+N more"

**Technical Notes:**
- Component: `<MediaExpandedView>` rendered in expanded island
- Layout: flex column with `gap-2` (8px spacing)
- Album art: `<img>` with `w-[60px] h-[60px] rounded-lg object-cover`
- Progress bar: custom `<ProgressBar>` component
- Responsive: expanded view width is min(380px, screen_width - 40px)

**Priority:** P0

#### US-3.2.3 — Animated Text Marquee for Long Titles
**As an** end user
**I want** long song titles to scroll/marquee in the pill
**so that** I can read the full title even when it exceeds the pill width (like Apple's implementation).

**Acceptance Criteria:**
- When text width exceeds available space, text scrolls horizontally (auto-scroll, not user-triggered)
- Auto-scroll starts after 2 second delay
- Scroll speed: 30px/s
- Text scrolls to the end, pauses for 1 second, then scrolls back
- Loops continuously while the title is displayed
- Animation is smooth (60fps) and does not affect other pill animations

**Edge Cases:**
- Multiple rapid text changes → reset animation and restart
- Text fits within space → no scrolling, static display
- Paused state → scrolling continues (title is still visible)

**Technical Notes:**
- Use Motion's `motion.div` with `animate={{ x: [0, -textWidth + containerWidth] }}` and `transition={{ duration: calculateDuration(textWidth), repeat: Infinity, repeatDelay: 1 }}`
- Calculate text width using a hidden measurement span
- Container: `overflow: hidden; white-space: nowrap;`
- Alternative: CSS animation with `@keyframes marquee` (simpler but less control)

**Priority:** P2

#### US-3.2.4 — Progress Bar
**As an** end user
**I want** to see a progress bar showing my current position in the track
**so that** I know how much of the song has played.

**Acceptance Criteria:**
- Thin horizontal bar (2px height) fills from left to right as track progresses
- Bar color: white (or accent color in dark theme)
- Background track: dark gray (30% opacity white)
- Position label: "1:23 / 4:56" format (mm:ss / mm:ss)
- Bar updates in real-time (every 1 second)
- Bar is interactive: click to seek (if player supports seeking)

**Edge Cases:**
- Track longer than 99:59 → show h:mm:ss format
- Position not available → show indeterminate progress animation
- Seek not supported → bar is non-interactive

**Technical Notes:**
- Component: `<ProgressBar current={position_us} total={length_us} />`
- Layout: label above or below the bar, or overlaid
- Click position → calculate percentage → send Seek IPC command
- Width: `current / length * 100%`
- Transition: use `transition: width 1s linear` for smooth updates

**Priority:** P1

---

## Feature 3.3: Playback Controls

**Description:** Implement play/pause, next track, previous track, and seek controls in the expanded Dynamic Island view. Controls communicate with media players via MPRIS D-Bus calls.

**Acceptance Criteria:** Play/Pause toggles playback. Next/Previous skips tracks. Seek by clicking progress bar. Controls reflect player state accurately. Disabled controls are grayed.

**Dependencies:** 3.1

**Notes:** Controls call MPRIS D-Bus methods: `PlayPause`, `Next`, `Previous`, `Seek`, `SetPosition`.

### User Stories

#### US-3.3.1 — Play/Pause Toggle
**As an** end user
**I want** to be able to play or pause my music from the Dynamic Island
**so that** I don't need to switch to the media app.

**Acceptance Criteria:**
- Button toggles between play and pause depending on current state
- Playing → shows pause icon, clicking pauses
- Paused → shows play icon, clicking plays
- Button state updates immediately on user click (optimistic UI)
- Actual D-Bus call happens asynchronously; if it fails, button reverts

**Edge Cases:**
- Player does not support pause → button hidden (check `CanPause`)
- Player stopped → button shows play, clicking starts playback
- Multiple players → controls affect the last active player (the one that sent the most recent event)

**Technical Notes:**
- IPC command: `{ "type": "MediaPlayPause", "payload": { "player": "org.mpris.MediaPlayer2.spotify" } }`
- Daemon calls `org.mpris.MediaPlayer2.Player.PlayPause`
- Optimistic update: UI toggles button state immediately, reverts on error (timeout 2s)
- Error handling: if D-Bus call fails, show brief error indicator on button

**Priority:** P0

#### US-3.3.2 — Next/Previous Track
**As an** end user
**I want** to skip to the next or previous track from the Dynamic Island
**so that** I can change songs without switching applications.

**Acceptance Criteria:**
- Two buttons: skip previous (⏮) and skip next (⏭)
- Clicking next → calls `Next` on the MPRIS player
- Clicking previous → calls `Previous` on the MPRIS player
- Buttons are disabled (grayed) if player reports `!CanGoNext` / `!CanGoPrevious`
- Double-click previous may restart current track (player-dependent)

**Edge Cases:**
- Player does not support next/prev → buttons hidden
- At start of playlist, previous does nothing → log, no error shown
- Rapid clicking → each click sends a separate command (debounce not needed, but queue)

**Technical Notes:**
- IPC commands: `MediaNext`, `MediaPrevious`
- Button layout: [⏮] [⏯] [⏭] with equal spacing
- Use simple SVG icons or Unicode symbols
- Disabled state: `opacity: 40%`, `cursor: not-allowed`, `pointer-events: none`

**Priority:** P0

#### US-3.3.3 — Seek via Progress Bar
**As an** end user
**I want** to click or drag on the progress bar to seek to a specific position in the track
**so that** I can jump to any part of the song.

**Acceptance Criteria:**
- Clicking on progress bar → seek to that position
- Dragging on progress bar → real-time preview of position, seek on release
- Seeks call `SetPosition` (absolute) or `Seek` (relative offset) depending on player support
- Thumbnail indicator (small circle) on progress bar when dragging
- Button hidden if `!CanSeek`

**Edge Cases:**
- Seek beyond track length → clamp to end
- Seek is faster than server response → optimistic position update, revert on error
- Live streams (no length) → seek bar hidden

**Technical Notes:**
- Use `onPointerDown` + `onPointerMove` + `onPointerUp` for drag handling
- Calculate seek position: `clickX / barWidth * trackLength`
- Send IPC command: `MediaSeekAbsolute { position_us }` or `MediaSeek { offset_us }`
- For live streams (length = 0), hide seek bar entirely

**Priority:** P1

#### US-3.3.4 — Volume Control (Optional)
**As an** end user
**I want** to adjust media volume from the Dynamic Island
**so that** I can control playback volume without switching apps.

**Acceptance Criteria:**
- Volume slider (or up/down buttons) in expanded media view
- Controls the player's volume via MPRIS `Volume` property
- If player does not expose volume, use system volume instead (via PipeWire/WirePlumber)
- Volume percentage displayed numerically (optional)

**Edge Cases:**
- Player does not support volume → fall back to system volume
- Volume property may not fire change signals → poll every 2 seconds when visible

**Technical Notes:**
- MPRIS: `org.mpris.MediaPlayer2.Player.Volume` (double, 0.0–1.0)
- System volume: PipeWire `org.pipewire.*` or ALSA via `alsactl`
- Volume slider: `<input type="range">` or custom slider component

**Priority:** P3

---

## Feature 3.4: Album Art Rendering

**Description:** Fetch, cache, and display album art in the expanded Dynamic Island view. Handle remote URLs, data URIs, and missing art gracefully.

**Acceptance Criteria:** Album art displays with correct aspect ratio. Remote URLs are fetched and cached. Data URIs are decoded and rendered. Missing art shows a placeholder. Transitions between art images are smooth.

**Dependencies:** 3.2

**Notes:** Album art URLs may be remote (HTTP), data URIs (base64), or local file paths.

### User Stories

#### US-3.4.1 — Art URL Fetching & Caching
**As a** system
**I want** to fetch album art from URLs and cache them locally
**so that** art displays quickly and does not cause repeated network requests.

**Acceptance Criteria:**
- Remote HTTP(S) URLs are fetched with a 5-second timeout
- Responses are cached to `$XDG_CACHE_HOME/dyd/album-art/{hash}.jpg`
- Cache has a maximum size of 50MB, LRU eviction
- Cache key: SHA256 hash of the URL
- Data URIs are decoded and cached in the same way (without fetching)
- Local file paths are copied to cache

**Edge Cases:**
- Network error → show placeholder, retry on next track
- URL changes (streaming with per-song art) → fetch and cache new art
- Corrupted cache file → delete and re-fetch
- Cache directory does not exist → create

**Technical Notes:**
- Use `reqwest` for HTTP fetching (in daemon)
- Art fetched by daemon, served to UI via IPC (base64-encoded in event payload, or file path)
- For local files: read and base64-encode, send in event payload (for small files) or write to shared temp file
- LRU cache: implement with `lru` crate or manual HashMap + access timestamps
- Cache path: `$XDG_CACHE_HOME/dyd/album-art/{hash}.{ext}`

**Priority:** P1

#### US-3.4.2 — Art Rendering in UI
**As an** end user
**I want** the album art to be displayed in the expanded view
**so that** I can visually identify the current track.

**Acceptance Criteria:**
- Art image is 60×60px rounded rectangle in expanded view
- Image uses `object-fit: cover` to fill the area without distortion
- Image loads with a 150ms fade-in transition
- Fallback: gradient placeholder with music note icon
- High-resolution art (>200px) is downscaled for performance

**Edge Cases:**
- Image fails to load → show placeholder
- Image loads slowly → show placeholder with loading spinner
- Transparent PNG art → render correctly over dark background

**Technical Notes:**
- `<img>` with `onLoad`/`onError` handlers
- Fade-in: `opacity: 0 → 1` with 150ms transition
- Placeholder: CSS gradient (dark blue/purple) + centered music note SVG
- Image received via IPC as base64 data URI or file URL

**Priority:** P1

#### US-3.4.3 — Art Transition Animation
**As an** end user
**I want** the album art to transition smoothly when the track changes
**so that** the experience feels polished.

**Acceptance Criteria:**
- When track changes, old art fades out (100ms) → new art fades in (150ms)
- Cross-fade: old art opacity → 0, new art opacity → 1, overlapping
- If same image (same URL) → no animation (detect by cached hash)
- Animation is interruptible (rapid track changes handled)

**Edge Cases:**
- New track has no art → fade out old art, fade in placeholder
- Rapid changes (gallery mode) → only last art animates in

**Technical Notes:**
- Use `AnimatePresence` with `key` set to art URL hash
- Wrap art image in `motion.div` with `initial={{ opacity: 0 }}`, `animate={{ opacity: 1 }}`, `exit={{ opacity: 0 }}`
- Transition duration: 150ms

**Priority:** P2

---

## Feature 3.5: Multi-Source Media

**Description:** Support multiple simultaneous media sources (e.g., Spotify + Firefox + Rhythmbox) with seamless switching between them. The most recently active source is shown by default.

**Acceptance Criteria:** Multiple media players are tracked simultaneously. Last-active-player wins display. User can switch between players via expanded view. Disappearing player is auto-removed.

**Dependencies:** 3.1, 3.2

**Notes:** When multiple players are active, show the one that most recently changed state (e.g., user just clicked play on Spotify, so show Spotify).

### User Stories

#### US-3.5.1 — Last-Active Player Selection
**As a** system
**I want** to display the media player that most recently changed state
**so that** the island shows the player the user is currently interacting with.

**Acceptance Criteria:**
- When multiple players are active, the one with the most recent `PlaybackStatus` or `Metadata` change is displayed
- Player changes are shown with a label: "Spotify • Song Title"
- Last-active tracking resets per session (not persisted)
- If the active player stops playback (Stopped state), fall to next most recent active player

**Edge Cases:**
- Two players change state at the exact same timestamp → use player name alphabetical order
- All players stopped → show "No media playing" message, return to dormant

**Technical Notes:**
- Each player has a `last_active: Instant` field updated on any event
- When selecting which player to display: sort by `last_active` descending, pick first with `PlaybackStatus == Playing`
- If none playing, pick the most recent Paused player

**Priority:** P1

#### US-3.5.2 — Player Switcher in Expanded View
**As an** end user
**I want** to switch between active media players from the expanded view
**so that** I can control any currently playing source.

**Acceptance Criteria:**
- Expanded view shows a small player selector (dropdown or horizontal scroll of player icons) when multiple players are active
- Clicking a player name switches the display to that player
- Controls and metadata update to reflect the selected player
- Player selector shows: player icon + short name (e.g., "Si" for Spotify)

**Edge Cases:**
- Only one player active → selector hidden
- Player disappears while selected → auto-switch to next available

**Technical Notes:**
- Player selector: horizontal row of small chips (24×24px) with player icons
- Selected player highlighted (white border/glow)
- Click → send IPC command `MediaSelectPlayer { player_name }`
- Player icons: simple SVGs per known player (Spotify, Firefox, etc.)

**Priority:** P2

#### US-3.5.3 — Player Identity Detection
**As a** system
**I want** to detect the human-readable name and icon for each MPRIS player
**so that** the UI can show which app is playing media.

**Acceptance Criteria:**
- Detect player identity via `org.mpris.MediaPlayer2.Identity` property
- Map known identities to icons:
  - "spotify" / "Spotify" → Spotify icon
  - "Firefox" → Firefox icon
  - "Chromium" / "google-chrome" → Chrome icon
  - "rhythmbox" → Rhythmbox icon
  - "vlc" / "VLC media player" → VLC icon
  - Unknown → generic music note icon
- Icons are 16×16px SVG files bundled in the UI

**Edge Cases:**
- Player identity changes (unlikely but possible) → update stored identity
- Unknown player → log identity for future mapping, show generic icon

**Technical Notes:**
- Identity → icon mapping in TypeScript:
  ```typescript
  const PLAYER_ICONS: Record<string, string> = {
    'spotify': 'spotify-icon',
    'firefox': 'firefox-icon',
    'chromium': 'chrome-icon',
    'vlc': 'vlc-icon',
  }
  ```
- Match case-insensitively against the identity string

**Priority:** P2
