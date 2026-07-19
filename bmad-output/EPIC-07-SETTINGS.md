# Epic 7: Settings & Preferences

## Objective
Build a comprehensive settings system with a UI panel accessible from the Dynamic Island. Users configure every aspect of the application: appearance, behavior, features, plugins, and accessibility.

## Business Value
Settings are essential for a production-quality application. Users have different preferences for notification behavior, appearance, and feature usage. A well-designed settings UI conveys professionalism and user-friendliness.

## Success Criteria
- Settings panel accessible from context menu (long-press on pill)
- Settings organized into logical categories
- All settings have sensible defaults
- Changes take effect immediately (no restart required for most)
- Settings persist across sessions
- Settings can be imported/exported as JSON

## Risks
- Scope creep — settings can become a full preferences window
- Backward compatibility when settings schema changes
- Some settings require daemon restart (e.g., D-Bus listener toggles)

## Dependencies
- Epic 1 (config persistence)
- Epic 2 (context menu for access)

## Complexity
⭐⭐⭐ — Moderate. Largely UI work with well-defined backend storage.

---

## Feature 7.1: Settings UI

**Description:** Build a settings panel UI accessible from the Dynamic Island context menu. Settings are organized into categories with clear descriptions and real-time previews where applicable.

**Acceptance Criteria:** Settings panel opens from long-press menu. Categories visible. Settings reflect current state. Changes apply immediately. Panel scrollable. "Reset All" button available.

**Dependencies:** 2.3 (context menu)

**Notes:** Settings panel can be a narrow expanded view or a separate full window. Prefer expanded view for v1 (stays within the island metaphor).

### User Stories

#### US-7.1.1 — Settings Panel Access
**As an** end user
**I want** to open settings from the Dynamic Island context menu
**so that** I can configure the application.

**Acceptance Criteria:**
- Long-press on pill → context menu → "Settings" option
- Settings panel opens as an expanded view (full island height)
- Close button (X) or swipe down to dismiss
- Settings panel is scrollable if content exceeds visible area
- Keyboard shortcut: no shortcut in v1 (future)

**Edge Cases:**
- Settings panel should not interfere with activity queue (pause cycling while settings are open)
- Opening settings while notification is active → notification continues in background

**Technical Notes:**
- Context menu option: `{ label: "Settings", icon: gear, action: "open_settings" }`
- Settings panel is a React component rendered in the expanded view slot
- IPC event: `OpenSettings` → UI transitions to settings view
- Back button in settings returns to previous state

**Priority:** P1

#### US-7.1.2 — Settings Categories
**As an** end user
**I want** settings organized into clear categories
**so that** I can find the setting I'm looking for quickly.

**Acceptance Criteria:**
- Categories:
  - General: start on login, language, theme
  - Notifications: DND mode, suppress native, per-app settings
  - Media: preferred player, album art, position tracking
  - System: battery, WiFi, bluetooth, volume, brightness toggles
  - Status Bar: icon order, visible icons
  - Plugins: plugin list, enable/disable (redirects to plugin management)
  - Accessibility: reduce motion, high contrast, font size
  - About: version, licenses, check for updates
- Each category is a collapsible section or a separate page

**Edge Cases:**
- Category with no settings (hidden features) → hidden

**Technical Notes:**
- Component: `<SettingsPanel>` with category navigation
- Categories as tabs (left sidebar) or accordion sections
- Persist last-opened category

**Priority:** P1

#### US-7.1.3 — Settings Persistence
**As a** system
**I want** settings to be saved to disk and loaded on startup
**so that** user preferences are preserved across sessions.

**Acceptance Criteria:**
- Settings stored in daemon-side config file: `$XDG_CONFIG_HOME/dyd/config.json`
- Settings loaded on daemon startup
- Setting change → write to config file
- Write is debounced (max 1 write per 500ms)
- Corrupt config file → merge with defaults, log warning
- Atomic write (write to temp, rename) to prevent corruption

**Edge Cases:**
- Config directory does not exist → create
- Config file unreadable → use defaults, log warning
- Config from newer version with unknown fields → preserve unknown fields

**Technical Notes:**
- Config struct: `#[derive(Serialize, Deserialize)]` with `#[serde(deny_unknown_fields)]`
- Use `serde_json` with atomic write
- Config path from `dirs::config_dir()`, then `dyd/config.json`
- IPC: `GetSettings` → returns full config | `UpdateSettings { changes }` → apply partial update

**Priority:** P1

---

## Feature 7.2: Theme System

**Description:** Implement a theme system supporting dark mode (default), light mode, and system-follow. Themes apply to the Dynamic Island appearance including colors, blur, and animations.

**Acceptance Criteria:** Theme selector in settings. Dark and light themes implemented. System-follow detects GNOME theme. Theme changes apply immediately without restart.

**Dependencies:** 7.1

**Notes:** Dark theme is the default (matches Apple's Dynamic Island). Light theme adjusts pill color to dark gray-on-light. System-follow uses `org.freedesktop.appearance.color-scheme` D-Bus property.

### User Stories

#### US-7.2.1 — Theme Selection
**As an** end user
**I want** to choose between dark, light, and system themes
**so that** the Dynamic Island matches my desktop appearance preference.

**Acceptance Criteria:**
- Three options: Dark (default), Light, Follow System
- Dark: black pill (#000), white text (#fff)
- Light: dark gray pill (#e5e5e7 or similar), black text (#000)
- Follow System: reads GNOME color scheme preference
- Theme changes immediately (no restart)

**Edge Cases:**
- System theme changes while app is running → detect via D-Bus signal, apply

**Technical Notes:**
- Theme context in React: `ThemeContext` provides `theme: 'dark' | 'light'`
- CSS variables for theming:
  ```css
  [data-theme="dark"] { --pill-bg: #000; --text: #fff; }
  [data-theme="light"] { --pill-bg: #e5e5e7; --text: #000; }
  ```
- System theme: listen to `org.freedesktop.appearance.color-scheme` on `org.freedesktop.portal.Desktop`

**Priority:** P2

#### US-7.2.2 — Accessibility: Reduce Motion
**As an** end user
**I want** to reduce or disable animations
**so that** the application is comfortable for users with motion sensitivity.

**Acceptance Criteria:**
- "Reduce Motion" toggle in Accessibility settings
- When enabled:
  - All animations are instant (0ms duration) or use fade-only
  - No spring animations, no slide animations, no scale animations
  - State transitions happen immediately
  - Hover effects disabled
- Setting follows system `prefers-reduced-motion` by default
- User can override system preference

**Edge Cases:**
- Transition from reduced-motion to full motion → animations resume
- System `prefers-reduced-motion` changes → setting can override

**Technical Notes:**
- Detect: `window.matchMedia('(prefers-reduced-motion: reduce)')`
- Motion component: check `settings.reduceMotion` and set `transition={{ duration: 0 }}` or `animation={undefined}`

**Priority:** P2

---

## Feature 7.3: Feature Toggles

**Description:** Allow users to individually enable or disable each feature (battery, WiFi, Bluetooth, volume, brightness, media, notifications, each plugin). Disabled features do not start their D-Bus listeners.

**Acceptance Criteria:** Each feature has a toggle in settings. Disabled feature does not consume D-Bus resources. Feature state persisted. Toggling a feature starts/stops its daemon listener.

**Dependencies:** 7.1

**Notes:** Feature toggles allow users to reduce resource usage and customize their experience.

### User Stories

#### US-7.3.1 — Feature Toggle Implementation
**As an** end user
**I want** to disable features I don't use
**so that** I can reduce resource usage and simplify the Dynamic Island.

**Acceptance Criteria:**
- Toggle for each feature: Battery, WiFi, Bluetooth, Volume, Brightness, Media, Notifications
- When disabled:
  - Daemon does not start listeners for that feature
  - No IPC events are sent for that feature
  - Feature icon is not shown in the status row
- When enabled:
  - Daemon starts listener
  - Icon appears in status row
- Toggles take effect immediately (no restart needed)

**Edge Cases:**
- Disabling a feature while it has an active activity → activity is dismissed
- Rapid toggling → debounce start/stop (500ms)

**Technical Notes:**
- Feature: `HashMap<FeatureId, bool>` in config
- FeatureId enum: `Battery`, `Wifi`, `Bluetooth`, `Volume`, `Brightness`, `Media`, `Notifications`
- Daemon: on config change, compare new to old, start/stop listeners accordingly

**Priority:** P2

#### US-7.3.2 — Per-App Notification Settings
**As an** end user
**I want** to configure notification behavior per application
**so that** I can allow notifications from messaging apps but block newsletter apps.

**Acceptance Criteria:**
- Per-app settings accessible from Notifications settings category
- Options per app:
  - Allow notifications (default)
  - Show silently (notification appears but no sound)
  - Show in queue only (no urgency boost)
  - Block
- App list populated from observed notification sources
- Changes apply immediately to subsequent notifications

**Edge Cases:**
- New app sends first notification → default behavior (Allow)
- App blocked → notification is dropped (not queued)

**Technical Notes:**
- Per-app config: `HashMap<String, AppNotificationConfig>` in config
- AppNotificationConfig: `{ allow: bool, show_silently: bool, urgency_boost: bool }`
- Filtering in daemon before queuing

**Priority:** P3

---

## Feature 7.4: Keyboard Shortcuts

**Description:** Provide keyboard shortcuts for common actions: toggle DND, open settings, cycle activities, dismiss current activity. Shortcuts are configurable.

**Acceptance Criteria:** Default shortcuts defined. Shortcuts work when Dynamic Island is focused and optionally as global shortcuts. Shortcut customization in settings.

**Dependencies:** 7.1

**Notes:** Global shortcuts require Tauri's global shortcut API or a dedicated D-Bus shortcut consumer. Prefer per-window shortcuts initially.

### User Stories

#### US-7.4.1 — Default Keyboard Shortcuts
**As an** end user
**I want** keyboard shortcuts for common Dynamic Island actions
**so that** I can use the application without a mouse.

**Acceptance Criteria:**
- Default shortcuts (when island is focused):
  - `Space` or `Enter`: expand current activity
  - `Escape`: collapse/close expanded view
  - `Tab` / `Shift+Tab`: cycle through interactive elements
  - `Left` / `Right`: cycle between activities in queue
  - `D`: toggle DND
  - `S`: open settings
- Global shortcuts (configurable):
  - `Super+Shift+D`: toggle DND (global)

**Edge Cases:**
- Shortcut conflicts with other applications → user customizable
- Island not focused → global shortcuts still work

**Technical Notes:**
- Use `onKeyDown` on the window for per-window shortcuts
- Global shortcuts: Tauri global-shortcut plugin
- Shortcut registration: `tauri-plugin-global-shortcut`

**Priority:** P3

#### US-7.4.2 — Shortcut Customization
**As an** end user
**I want** to customize keyboard shortcuts
**so that** I can avoid conflicts with my other applications.

**Acceptance Criteria:**
- Settings → Keyboard Shortcuts section
- List of all actions with current shortcut
- Click to change → press key combination → displayed
- Validation: duplicate shortcut warning
- Reset to defaults button
- Shortcuts persisted in config

**Edge Cases:**
- Unassigned shortcut → action not available via keyboard
- Invalid shortcut (bare modifier key) → ignore

**Technical Notes:**
- Shortcut storage: `HashMap<String, String>` (action → key combo)
- Key combo format: `"Ctrl+Shift+D"` or `"Super+D"`
- Component: `<ShortcutRecorder>` captures keydown events and displays combo

**Priority:** P3

---

## Feature 7.5: Accessibility Settings

**Description:** Implement accessibility features: screen reader support (ARIA), high contrast mode, font size scaling, and reduced motion (7.2.2).

**Acceptance Criteria:** ARIA labels on all interactive elements. High contrast mode increases contrast ratios. Font size scaling from 75% to 150%. Settings UI is keyboard-accessible. Screen reader announces state changes.

**Dependencies:** 7.1

**Notes:** Accessibility is not optional. Even v1 must meet basic WCAG 2.1 AA standards.

### User Stories

#### US-7.5.1 — ARIA & Screen Reader Support
**As a** visually impaired user
**I want** all Dynamic Island elements to have proper ARIA labels and roles
**so that** I can use the application with a screen reader.

**Acceptance Criteria:**
- Pill: `role="status"`, `aria-live="polite"`, `aria-label="Dynamic Island: [current state description]"` updating on state changes
- Expanded view: `role="dialog"`, `aria-modal="true"`, `aria-label="[content title]"`
- Buttons: `aria-label` describing action (e.g., "Play/Pause music")
- Progress bar: `role="progressbar"`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`
- State changes announced: "New notification from Slack", "Media playing: Song Name"
- Focus management: focus trapped in expanded view, returned to pill on close

**Edge Cases:**
- Rapid state changes → debounce announcements (max 1 per 2 seconds)
- Screen reader already reading → new announcement queues

**Technical Notes:**
- Use `aria-live="polite"` on the pill container
- For state change announcements, use a visually hidden `aria-live="assertive"` region
- Update `aria-label` on pill whenever current activity changes
- Test with Orca (GNOME screen reader)

**Priority:** P2

#### US-7.5.2 — High Contrast Mode
**As a** visually impaired user
**I want** a high contrast mode with stronger color differentiation
**so that** I can perceive the Dynamic Island content.

**Acceptance Criteria:**
- High contrast mode toggle in Accessibility settings
- Changes:
  - Pill background: pure white (#FFF) or pure black (#000) depending on theme
  - Text: pure white on black, pure black on white
  - Borders: 1px visible border around pill
  - Icons: filled, not outlined
  - Contrast ratio: minimum 7:1 for all text (WCAG AAA)
- Setting follows system `prefers-contrast: more` by default

**Edge Cases:**
- High contrast + reduced motion → both respected

**Technical Notes:**
- CSS variables: `--contrast-border` added in high contrast mode
- Avoid transparency in high contrast mode (solid backgrounds)

**Priority:** P2

#### US-7.5.3 — Font Size Scaling
**As an** visually impaired user
**I want** to adjust the font size of Dynamic Island content
**so that** I can read text more easily.

**Acceptance Criteria:**
- Font size slider: 75% to 150% (default: 100%)
- All text elements scale: pill text, expanded view text, button labels
- Icons do not scale (remain fixed size)
- Font size respects system accessibility settings by default

**Edge Cases:**
- Very large font (150%) → text may overflow pill width → truncation still works
- Layout integrity: expanded view should not break at any font size

**Technical Notes:**
- CSS: `font-size: var(--font-scale) * base-size`
- Use `rem` units for text, controlled by a CSS custom property
- Store scale factor in config

**Priority:** P3
