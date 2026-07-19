# Style Guide — dyland

## Design Principles

1. **Glanceable first** — every element must be readable in under 1 second
2. **Subtlety over spectacle** — no unnecessary gradients, shadows, or flourishes
3. **Physical plausibility** — animations mimic real-world spring physics
4. **Consistency** — one way to do everything, repeated everywhere
5. **Dark-first** — dark theme is default and primary; light theme is adaptation
6. **System alignment** — use system fonts, system icons, system colors where possible

---

## Color Palette

### Dark Theme (Default)

```
Pill Background     #000000     rgba(0, 0, 0, 1)
Pill Border         none
Text Primary        #FFFFFF     rgba(255, 255, 255, 1)
Text Secondary      #EBEBF5     rgba(235, 235, 245, 0.6)
Text Tertiary       #EBEBF5     rgba(235, 235, 245, 0.3)
Icon Primary        #FFFFFF     rgba(255, 255, 255, 1)
Icon Secondary      #EBEBF5     rgba(235, 235, 245, 0.5)
Divider             #FFFFFF     rgba(255, 255, 255, 0.15)
Button Active       #FFFFFF     rgba(255, 255, 255, 0.15)
Button Hover        #FFFFFF     rgba(255, 255, 255, 0.25)
Backdrop Blur       —           backdrop-filter: blur(20px)
                     —           background: rgba(0, 0, 0, 0.6) [fallback]

Context Menu BG     #1C1C1E     rgba(28, 28, 30, 0.95)
Context Menu Blur   —           backdrop-filter: blur(40px)

Battery Low         #FF453A     rgba(255, 69, 58, 1)  /* red */
Battery Charging    #30D158     rgba(48, 209, 88, 1)  /* green */
DND Active          #FF9F0A     rgba(255, 159, 10, 1) /* yellow */
Critical Urgency    #FF453A     rgba(255, 69, 58, 1)  /* red */
Accent (generic)    #007AFF     rgba(0, 122, 255, 1)  /* blue */
```

### Light Theme

```
Pill Background     #E5E5E7     rgba(229, 229, 231, 1)  /* light gray */
Pill Border         #D1D1D6     rgba(209, 209, 214, 1)  /* subtle border */
Text Primary        #000000     rgba(0, 0, 0, 1)
Text Secondary      #3A3A3C     rgba(58, 58, 60, 0.6)
Text Tertiary       #3A3A3C     rgba(58, 58, 60, 0.3)
Icon Primary        #000000     rgba(0, 0, 0, 1)
Icon Secondary      #3A3A3C     rgba(58, 58, 60, 0.5)
Divider             #000000     rgba(0, 0, 0, 0.12)
Button Active       #000000     rgba(0, 0, 0, 0.1)
Button Hover        #000000     rgba(0, 0, 0, 0.18)
Backdrop Blur       —           backdrop-filter: blur(20px)
                     —           background: rgba(229, 229, 231, 0.8) [fallback]

Context Menu BG     #F2F2F7     rgba(242, 242, 247, 0.95)
Context Menu Blur   —           backdrop-filter: blur(40px)

Battery Low         #FF3B30     rgba(255, 59, 48, 1)
Battery Charging    #34C759     rgba(52, 199, 89, 1)
DND Active          #FF9500     rgba(255, 149, 0, 1)
Critical Urgency    #FF3B30     rgba(255, 59, 48, 1)
Accent              #007AFF     rgba(0, 122, 255, 1)
```

### Design Tokens (CSS Custom Properties)

```css
:root[data-theme="dark"] {
  --pill-bg: #000000;
  --pill-border: none;
  --text-primary: #ffffff;
  --text-secondary: rgba(235, 235, 245, 0.6);
  --text-tertiary: rgba(235, 235, 245, 0.3);
  --icon-primary: #ffffff;
  --icon-secondary: rgba(235, 235, 245, 0.5);
  --divider: rgba(255, 255, 255, 0.15);
  --button-active: rgba(255, 255, 255, 0.15);
  --button-hover: rgba(255, 255, 255, 0.25);
  --backdrop-filter: blur(20px);
  --backdrop-fallback: rgba(0, 0, 0, 0.6);
  --menu-bg: rgba(28, 28, 30, 0.95);
  --battery-low: #ff453a;
  --battery-charging: #30d158;
  --dnd-active: #ff9f0a;
  --critical: #ff453a;
  --accent: #007aff;
}

:root[data-theme="light"] {
  --pill-bg: #e5e5e7;
  --pill-border: 1px solid #d1d1d6;
  --text-primary: #000000;
  --text-secondary: rgba(58, 58, 60, 0.6);
  --text-tertiary: rgba(58, 58, 60, 0.3);
  --icon-primary: #000000;
  --icon-secondary: rgba(58, 58, 60, 0.5);
  --divider: rgba(0, 0, 0, 0.12);
  --button-active: rgba(0, 0, 0, 0.1);
  --button-hover: rgba(0, 0, 0, 0.18);
  --backdrop-filter: blur(20px);
  --backdrop-fallback: rgba(229, 229, 231, 0.8);
  --menu-bg: rgba(242, 242, 247, 0.95);
  --battery-low: #ff3b30;
  --battery-charging: #34c759;
  --dnd-active: #ff9500;
  --critical: #ff3b30;
  --accent: #007aff;
}
```

---

## Typography

### Font Stack

```
font-family: -apple-system, 'Segoe UI', Roboto, 'Noto Sans', 'Ubuntu', 'Cantarell',
             'Helvetica Neue', Arial, sans-serif;
```

System-first. No custom font loading — every font adds ~100ms to startup.

### Font Sizes

| Usage | Size | Weight | Line Height |
|---|---|---|---|
| Pill content text | 13px | 500 (medium) | 1.3 |
| Expanded title | 16px | 600 (semibold) | 1.3 |
| Expanded body | 14px | 400 (normal) | 1.4 |
| Labels / Captions | 12px | 400 (normal) | 1.3 |
| Timestamp (relative) | 11px | 400 (normal) | 1.2 |
| Button labels | 13px | 500 (medium) | 1.0 |
| App name (in notification) | 13px | 600 (semibold) | 1.3 |
| Progress bar label | 11px | 400 (normal) | 1.2 |

### Truncation

- Single line: `text-overflow: ellipsis; overflow: hidden; white-space: nowrap;`
- Multi-line: `display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;`
- Maximum lines in body: 4 (scroll if longer)
- Marquee scrolling for titles that exceed pill width: 30px/s, 2s delay before start, 1s pause at end

---

## Spacing

4px base grid. All spacing values are multiples of 4.

| Token | Value | Usage |
|---|---|---|
| `--space-1` | 4px | Micro padding, icon gaps |
| `--space-2` | 8px | Inner padding, small gaps |
| `--space-3` | 12px | Default padding |
| `--space-4` | 16px | Content padding |
| `--space-5` | 20px | Section spacing |
| `--space-6` | 24px | Large gaps |
| `--space-8` | 32px | Expanded view margins |
| `--space-10` | 40px | Max content margin |

### Layout Grid

- Pill content: `padding: 0 var(--space-3)` (12px horizontal)
- Expanded view: `padding: var(--space-4)` (16px all sides)
- Between content rows: `var(--space-2)` (8px)
- Between sections: `var(--space-4)` (16px)
- Icon to text gap: `var(--space-1)` (4px) in pill, `var(--space-2)` (8px) in expanded

---

## The Pill

### Dimensions

| State | Width | Height | Corner Radius |
|---|---|---|---|
| Dormant | 126px | 36px | 18px (rounded-full) |
| Active (media) | 180px | 36px | 18px |
| Active (notification) | 200px | 36px | 18px |
| Active (status icons) | auto (fit content, max 300px) | 36px | 18px |
| Expanded | 380px | auto (min 120px, max 400px) | 18px (top), 0 (bottom corners optional) |

### Visual Spec

```
         ┌──────────── 126/180/380px ────────────┐
         │                                        │
         │  [icon]  Text content truncated…        │  ← 36px
         │                                        │
         └─────────── rounded-full ───────────────┘
                      ╲___18px__╱
```

- Pill is flush against the top edge of the screen (`y: 0`)
- Pill is horizontally centered (`left: 50%; transform: translateX(-50%)`)
- Expanded view drops down from pill, creating a unified shape
- No visible border, shadow, or outline on the pill (except light theme has subtle `1px solid #d1d1d6`)

---

## Icons

### Style

- **Type:** Outlined, uniform stroke width (1.5px)
- **Corner radius:** Round caps and joins (`stroke-linecap: round; stroke-linejoin: round`)
- **Size:** 16×16px in pill, 32×32px in expanded view
- **Color:** `var(--icon-primary)` or `var(--icon-secondary)`
- **Set:** Custom SVGs matching Apple SF Symbols style

### Naming Convention

```
icon-{category}-{state}
```

Examples: `icon-media-play`, `icon-media-pause`, `icon-battery-charging`, `icon-wifi-connected`, `icon-bluetooth-on`, `icon-volume-high`, `icon-dnd-on`, `icon-chevron-right`, `icon-x-close`

### Icon Set (Minimum Viable)

```
Media:        play, pause, next, previous, music-note, musical-notes
System:       battery-0, battery-25, battery-50, battery-75, battery-100,
              battery-charging, bolt,
              wifi-off, wifi-1, wifi-2, wifi-3, wifi-4,
              bluetooth-off, bluetooth-on,
              volume-off, volume-low, volume-medium, volume-high,
              brightness-low, brightness-high, sun-max, sun-min
Notifications: bell, bell-fill, bell-slash (DND),
               app-generic, x-close, chevron-left, chevron-right
Status:       checkmark, exclamationmark-triangle, arrow-down,
              ellipsis (loading), circle (dot indicator)
DND:          moon, moon-fill, moon-zzz
Misc:         gear (settings), square-and-arrow-up (share),
              arrow-clockwise (refresh), trash (dismiss)
```

---

## Animations

### Easing Curves

| Type | Curve | Usage |
|---|---|---|
| Spring expand | `stiffness: 400, damping: 28, mass: 0.8` | Pill size increase, view expansion |
| Spring collapse | `stiffness: 350, damping: 30, mass: 0.8` | Pill size decrease, view collapse |
| Content enter | `easeOut` (cubic-bezier 0.25, 0.1, 0.25, 1) | Content sliding/fading in |
| Content exit | `easeIn` (cubic-bezier 0.42, 0, 1, 1) | Content sliding/fading out |
| Linear | `linear` | Progress bar fill |
| Backdrop blur | `easeOut` 200ms | Blur/frosted glass transition |

### Duration Reference

| Animation | Duration | Notes |
|---|---|---|
| Dormant ↔ Active | 200-300ms | Spring-based |
| Active ↔ Expanded | 300-400ms | Spring-based |
| Content enter | 200ms | Staggered: 50ms after size change |
| Content exit | 150ms | Before size change (reverse stagger) |
| Hover effect | 150ms | Fade in, 200ms fade out |
| Status icon switch | 100ms | Cross-fade between icons |
| Fullscreen hide/show | 100ms | Fast, utilitarian |
| Progress bar update | 1s linear | Smooth continuous fill |

### Animation Principles

1. **All animations use `transform` and `opacity` only** — never `width`, `height`, `top`, `left`, `margin`, `padding`
2. **Every animation is interruptible** — new event cancels current animation, starts from current visual state
3. **Staggered coordination** — content follows container (content starts 50ms after container starts expanding)
4. **Spring overshoot** — expand animations overshoot ~5% and settle back (feels organic)
5. **Reduced motion** — all animations disabled when `prefers-reduced-motion: reduce`
6. **60fps requirement** — any animation that drops below 55fps should be optimized

### Motion Implementation Pattern

```typescript
import { motion } from 'motion/react'

// Pill container
<motion.div
  layout
  animate={{
    width: isExpanded ? 380 : isActive ? 180 : 126,
    height: isExpanded ? expandedHeight : 36,
  }}
  transition={{
    type: 'spring',
    stiffness: isExpanded ? 300 : 400,
    damping: isExpanded ? 25 : 28,
    mass: 0.8,
  }}
>
  {/* Content */}
  <AnimatePresence mode="popLayout">
    {currentActivity ? (
      <motion.div
        key={currentActivity.id}
        initial={{ y: 10, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: -10, opacity: 0 }}
        transition={{ duration: 0.2, ease: 'easeOut' }}
      >
        <ActivityContent activity={currentActivity} />
      </motion.div>
    ) : (
      <motion.div key="status-icons" ...>
        <StatusIconRow />
      </motion.div>
    )}
  </AnimatePresence>
</motion.div>
```

---

## Component Design Specs

### Button (inside Expanded View)

```
 ┌──────────────────┐
 │   Button Label   │
 └──────────────────┘

- Height: 32px
- Horizontal padding: 12px
- Border radius: 8px
- Background: var(--button-active) (normal)
- Background: var(--button-hover) (hover)
- Text: var(--text-primary), 13px, 500 weight
- No outline border (use background change for state)
- Disabled: opacity 0.4, cursor not-allowed
```

### Slider (Volume / Brightness)

```
●━━━━━━━━━━━━━━━━○─────────
↑ active         ↑ inactive
track            track

- Track height: 4px
- Active track: var(--text-primary)
- Inactive track: var(--divider)
- Thumb: 12px circle, var(--text-primary)
- Thumb hover: scale 1.2
- Thumb active: scale 1.4
- Border radius: 2px (track)
- Click target: 24px touch area around slider
```

### Progress Bar (Media)

```
●━━━━━━━━━━━━━━━━━━━━━━○─────────────────────
↑ current position                        ↑ end
- Track height: 3px
- Filled portion: var(--text-primary)
- Remaining: var(--divider)
- Thumb: 8px circle (visible on hover/drag)
- No thumb on passive display (only on hover)
- Click to seek (if supported)
- Label underneath: "1:23 / 4:56" (11px, var(--text-tertiary))
```

### Context Menu

```
┌─────────────────────────┐
│  [icon]  Settings        │
│  [icon]  Do Not Disturb  │  ← toggle shows checkmark
│  [icon]  Disable Media   │
│  ─────────────────────── │
│  [icon]  Quit            │
└─────────────────────────┘

- Width: 200px (min) - 280px (max based on content)
- Background: var(--menu-bg) with backdrop-blur(40px)
- Corner radius: 14px
- Items: 36px height, 12px horizontal padding
- Icon: 16px, left-aligned
- Label: 14px, weight 400
- Separator: 1px var(--divider), 8px vertical margin
- Item hover: background var(--button-hover)
- Positioned below and centered relative to pill
- Offset: 4px gap below pill
```

---

## Window Spec (Tauri Configuration)

```json
{
  "decorations": false,
  "transparent": true,
  "always_on_top": true,
  "skip_taskbar": true,
  "focus": false,
  "width": 400,
  "height": 100,
  "min_width": 126,
  "min_height": 36,
  "max_width": 500,
  "max_height": 500,
  "resizable": false,
  "center": false,
  "x": "<calculated on startup>",
  "y": 0
}
```

The window is intentionally tall (100px) to allow the expanded view to render downward from the pill without needing to resize the window. The visible area is controlled by the React component layout — the WebView renders the full 400×100 area, but the pill sits at `y: 0` and expanded content renders below it within the same window.

---

## Accessibility

- All interactive elements: `role`, `aria-label`, `tabindex` required
- Pill: `role="status"`, `aria-live="polite"`, dynamic `aria-label`
- Expanded view: `role="dialog"`, `aria-modal="true"`, focus trap
- All icons: `aria-hidden="true"` (decorative)
- Color contrast: minimum 4.5:1 (AA) for all text, 7:1 (AAA) in high contrast mode
- Focus indicator: 2px solid white outline, 2px offset, rounded corners
- `:focus-visible` only — no focus ring on mouse click
- All gestures have keyboard alternatives
- Reduced motion: `@media (prefers-reduced-motion: reduce)` disables all animations

---

## Responsive Behavior

| Screen Width | Behavior |
|---|---|
| >800px | Normal pill at top-center |
| 400-800px | Pill centered, max expanded width = screen width - 20px |
| <400px | Pill at full width (minus 4px margins), pill height may increase |
| Ultra-wide (>2000px) | Max expanded width capped at 500px |

The Dynamic Island is a fixed-position overlay — it does not respond to viewport changes except for repositioning on monitor change.
