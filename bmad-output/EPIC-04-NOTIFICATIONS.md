# Epic 4: Notification Interception

## Objective
Intercept GNOME desktop notifications via D-Bus (`org.freedesktop.Notifications`) and display them in the Dynamic Island with actions, grouping, and Do Not Disturb support.

## Business Value
Notifications are the second-most-used Dynamic Island feature. Users want glanceable, actionable notifications without the GNOME notification center popup.

## Success Criteria
- Notifications are intercepted before GNOME shell shows them (or suppressed)
- Notification appears in Dynamic Island pill (icon + snippet)
- Expanded view shows full notification content with action buttons
- Reply actions (for messaging apps) trigger inline text input
- Notifications from the same app are grouped
- Do Not Disturb mode suppresses non-critical notifications
- Notification history is accessible

## Risks
- GNOME's notification daemon is `gnome-shell` itself — intercepting requires replacing or proxying the notification service
- Suppressing native notifications may make users miss alerts when dyland is not running
- Notification actions (buttons) have varying levels of D-Bus support
- Some apps require interactive reply (e.g., Telegram, WhatsApp) — complex to implement
- Accessibility: screen readers need notification events

## Dependencies
- Epic 1 (IPC, daemon)
- Epic 2 (pill rendering, expanded view)

## Complexity
⭐⭐⭐ — Moderate. D-Bus notification spec is straightforward; challenging part is intercepting and suppressing native notifications.

---

## Feature 4.1: D-Bus Notification Monitor

**Description:** Register the daemon as a notification consumer on `org.freedesktop.Notifications` to intercept all desktop notifications before they reach the GNOME shell. Optionally suppress the native notification popup.

**Acceptance Criteria:** Daemon receives all notifications from `org.freedesktop.Notifications`. Captures: app_name, title, body, urgency, actions, hints. Can suppress native notification display. Handles notification close signals.

**Dependencies:** 1.2

**Notes:** Two approaches: (a) proxy the notification bus — daemon registers on `org.freedesktop.Notifications`, forwards to GNOME after processing; (b) listen-only — daemon subscribes as a secondary listener, does not suppress native notifications. Approach (b) is safer initially.

### User Stories

#### US-4.1.1 — Notification Signal Subscription
**As a** system
**I want** to receive all desktop notification signals from `org.freedesktop.Notifications`
**so that** I can display them in the Dynamic Island.

**Acceptance Criteria:**
- Daemon connects to `org.freedesktop.Notifications` on the session bus
- Subscribes to `Notify` method calls via a proxy that intercepts or listens
- Captures all parameters: `app_name`, `replaces_id`, `app_icon`, `summary`, `body`, `actions`, `hints`, `expire_timeout`
- Received notifications are normalized into a common `Notification` struct
- Notifications are emitted to the UI via IPC within 100ms of receipt

**Edge Cases:**
- Notification `replaces_id` matches existing → update existing notification (don't create new)
- Empty body → show only summary
- Binary blob in hints (some apps abuse hints) → skip unparseable hint values

**Technical Notes:**
- Use `zbus` to implement a service on `org.freedesktop.Notifications` (approach: own the bus name)
- Or: use `monitor` mode — register as a secondary listener
- Notification struct:
  ```rust
  struct Notification {
      id: u32,
      replaces_id: u32,
      app_name: String,
      app_icon: String,  // icon name or path
      summary: String,
      body: String,
      actions: Vec<String>,  // alternating key, label
      hints: HashMap<String, Variant>,
      expire_timeout: i32,  // -1 = default, 0 = never
      timestamp: Instant,
  }
  ```

**Priority:** P0

#### US-4.1.2 — Native Notification Suppression
**As an** end user
**I want** native GNOME notification popups to be suppressed when dyland is running
**so that** I don't see duplicate notifications (one in Dynamic Island, one from GNOME).

**Acceptance Criteria:**
- When daemon is running and healthy, native GNOME notification popups are suppressed
- When daemon is not running, native notifications appear normally (fail-safe)
- Suppression is configurable (user may want both)
- Suppression only applies to notifications, not to other notification features (sounds, banners)

**Edge Cases:**
- Daemon crashes → native notifications should start appearing again automatically
- Notification sound should still play (suppression is visual only)

**Technical Notes:**
- Approach: daemon implements the `org.freedesktop.Notifications` service, taking the bus name from GNOME shell
- Daemon receives the `Notify` call, processes it, and does NOT forward to GNOME
- If daemon dies, bus name is released → GNOME shell re-acquires it → notifications work normally
- Sounds: pass through by calling GNOME's `org.gnome.Shell.Sound` or respecting `hints.sound-file`
- Feature flag: `suppress_native_notifications: bool` in settings

**Priority:** P1

#### US-4.1.3 — Notification Close Handling
**As a** system
**I want** to handle notification close events
**so that** dismissed notifications are tracked and not re-displayed.

**Acceptance Criteria:**
- When user dismisses a notification (via island or natively), daemon receives `NotificationClosed` signal
- Closed notification is removed from the activity queue
- `CloseReason` is logged for debugging (1 = expired, 2 = dismissed, 3 = programmatic)
- Daemon sends `CloseNotification` D-Bus call when user dismisses via island

**Edge Cases:**
- Notification expires naturally → remove from queue, no action
- App cancels its own notification (e.g., message read elsewhere) → remove from queue

**Technical Notes:**
- Implement `org.freedesktop.Notifications.CloseNotification` D-Bus method
- Emit `NotificationClosed` signal when user dismisses from island
- Track notification IDs to prevent stale entries

**Priority:** P1

---

## Feature 4.2: Notification Rendering in Island

**Description:** Render intercepted notifications in the Dynamic Island pill and expanded view. Pill shows app icon + truncated summary. Expanded view shows full content, actions, and large icon.

**Acceptance Criteria:** Active pill shows app icon + one-line summary. Expanded view shows app name, icon, full body text, and action buttons. Notification style matches Apple's notification appearance.

**Dependencies:** 2.1, 2.2, 4.1

**Notes:** Text length in pill should match media truncation behavior. Actions appear as text buttons in expanded view.

### User Stories

#### US-4.2.1 — Pill Notification Indicator
**As an** end user
**I want** incoming notifications to appear in the Dynamic Island pill
**so that** I can see them without a disruptive popup.

**Acceptance Criteria:**
- Active pill shows: [app icon (16×16px)] + [truncated summary/message]
- App icon uses the app's icon from the system icon theme
- If system icon not found, use a generic bell icon
- Urgency level affects visual treatment:
  - Low: no special treatment
  - Normal: standard display
  - Critical: pulsing glow, red dot indicator
- Notification replaces the previous pill content (goes to activity queue)

**Edge Cases:**
- Very long summary → truncate with ellipsis (max 25 chars)
- No summary → show app name
- App icon not found → use first letter of app name as fallback

**Technical Notes:**
- Component: `<PillNotificationContent>` rendered inside `<Pill>`
- Icon: use `libadwaita` icon lookup or parse app_icon hint field
- Icon rendering: always 16×16px, centered in a 20×20px container
- Urgency indicators: from `hints.urgency` (0=low, 1=normal, 2=critical)

**Priority:** P0

#### US-4.2.2 — Expanded Notification View
**As an** end user
**I want** to expand the Dynamic Island to see full notification details
**so that** I can read the complete message and take action.

**Acceptance Criteria:**
- Expanded view layout:
  - [Large app icon (32×32px)] + [App name (bold)] + [timestamp]
  - Full notification body text (wrapped, not truncated)
  - Action buttons (if any) in a horizontal row below body
  - Dismiss button (X) in top-right corner
- Body text supports: plain text only (GNOME notifications don't support rich text)
- Max height for body: 4 lines with scroll for overflow

**Edge Cases:**
- No body → show only summary (larger font)
- Very long body → scrollable content area
- Multiple notifications from same app → show count badge

**Technical Notes:**
- Component: `<NotificationExpandedView>`
- Timestamp: relative ("2m ago", "1h ago") using `Intl.RelativeTimeFormat`
- Body text: `whitespace: pre-wrap; word-break: break-word;`
- Actions: rendered as buttons, one per action (max 3 visible, overflow in menu)

**Priority:** P0

#### US-4.2.3 — App Icon Resolution
**As a** system
**I want** to display the correct icon for the app that sent the notification
**so that** users can identify the source at a glance.

**Acceptance Criteria:**
- Icon resolution priority:
  1. `app_icon` parameter (may be a file path)
  2. `hints.image-data` (raw image data)
  3. `hints.image-path` (file path to image)
  4. Desktop file icon lookup by app_name
  5. Generic fallback icon
- Resolved icon is cached (path → icon mapping)
- Icon is rendered at the appropriate size (16px pill, 32px expanded)

**Edge Cases:**
- `image-data` is a large raw pixel buffer → resize to 32×32
- Icon file is SVG → render correctly
- No icon found → fallback to colored circle with first letter

**Technical Notes:**
- Desktop file lookup: `$XDG_DATA_HOME/applications/` and `/usr/share/applications/`
- Parse `.desktop` file for `Icon=` field
- Icon theme lookup: use `libadwaita` or manual `gtk3-icon-browser` equivalent
- Cache resolved icons in `HashMap<String, CachedIcon>` with LRU eviction

**Priority:** P1

---

## Feature 4.3: Notification Actions

**Description:** Implement interactive notification action buttons in the expanded Dynamic Island view. Support both simple actions (dismiss, open app) and inline reply (for messaging apps).

**Acceptance Criteria:** Action buttons from notifications are rendered. Clicking an action calls the corresponding D-Bus method. Inline reply opens a text input field. Actions send `NotificationInvoked` signal.

**Dependencies:** 4.1, 4.2

**Notes:** Notification actions are defined as alternating key-value pairs in the `actions` parameter. Inline reply requires `inline-reply` hint and text input field.

### User Stories

#### US-4.3.1 — Action Button Rendering
**As an** end user
**I want** to see action buttons on notifications that support them
**so that** I can respond or interact directly from the Dynamic Island.

**Acceptance Criteria:**
- Action buttons appear as rounded text buttons below notification body in expanded view
- Max 3 buttons visible; if more, show "+N" overflow indicator
- Common actions localized: "Reply", "Mark as Read", "Archive", "Dismiss"
- Button labels match the notification's action labels (unmodified)
- Clicking a button sends `InvokeAction` IPC command with action key

**Edge Cases:**
- No actions → no buttons shown (normal state)
- Action key is hidden (starts with "__") → skip (internal action)
- Very long action label → truncate to 20 chars

**Technical Notes:**
- Actions array: `["key1", "Label 1", "key2", "Label 2", ...]`
- Render as: `{actions[1]}: {key: actions[0]}`, `{actions[3]}: {key: actions[2]}`
- Component: `<button>` with pill-style styling
- Use `onClick` → send IPC command → daemon calls `org.freedesktop.Notifications.InvokeAction` or `org.freedesktop.Notifications.ActionInvoked` signal

**Priority:** P1

#### US-4.3.2 — Inline Reply
**As an** end user
**I want** to reply to messages directly from the Dynamic Island
**so that** I can respond quickly without opening the messaging app.

**Acceptance Criteria:**
- Notifications with `inline-reply` hint show a text input field + send button
- Text input is a single-line field with submit-on-Enter
- Send button is disabled when input is empty
- Sent reply calls `org.freedesktop.Notifications.InvokeAction` with the reply text
- Input field shows placeholder text ("Reply…") matching Apple's style

**Edge Cases:**
- Very long reply → no client-side limit (server may truncate)
- Multiple inline-reply notifications → each has separate input state
- User cancels reply → dismiss by clicking outside or escape key

**Technical Notes:**
- Detect inline-reply: `hints.x-kde-reply-placeholder-text` or `hints.inline-reply`
- `InvokeAction` with action key and reply text as variant
- Input component: styled to match dark theme, placeholder text gray
- Use `autoFocus` when expanded view opens with inline-reply

**Priority:** P2

#### US-4.3.3 — Open App on Click
**As an** end user
**I want** clicking a notification (not on action buttons) to open the sending application
**so that** I can view the full context.

**Acceptance Criteria:**
- Clicking the notification body (expanded view, non-action area) opens the sending app
- App is activated via D-Bus `org.freedesktop.Application.Activate` or by launching the desktop file
- If app has a specific action (like opening a specific message), that action is invoked
- Fallback: launch the app binary from its desktop file

**Edge Cases:**
- App not installed → show error toast
- Desktop file not found → generic "open" attempt via `xdg-open`
- App already running → focus existing window (via `ActivateAction`)

**Technical Notes:**
- Desktop file identification: `desktop-entry` hint or app_name → search .desktop files
- D-Bus activation: `org.freedesktop.Application.{app_id}.Activate`
- Fallback: `std::process::Command::new("gtk-launch").arg(desktop_id)`

**Priority:** P1

---

## Feature 4.4: Notification Grouping

**Description:** Group notifications from the same app to avoid flooding the Dynamic Island. Show a count badge for grouped notifications. Group expands to show individual items.

**Acceptance Criteria:** Notifications from same app are grouped under one pill entry. Count badge shows number of notifications. Expanded view shows list of grouped notifications. User can expand/collapse the group.

**Dependencies:** 4.2

**Notes:** Match Apple's notification grouping behavior: chronologically ordered, most recent first.

### User Stories

#### US-4.4.1 — Automatic Grouping
**As a** system
**I want** notifications from the same app to be automatically grouped
**so that** the Dynamic Island is not overwhelmed by multiple notifications from one app.

**Acceptance Criteria:**
- Notifications from the same app with the same `app_name` are grouped
- Group shows: [app icon] + [app name] + [count badge "3"]
- Group appears as a single entry in the activity queue
- Group timeout: group auto-dismisses after 8 seconds of no new notifications from that app
- New notification from a grouped app → reset group timeout, increment count

**Edge Cases:**
- Group has mixed urgency → use highest urgency for group indicator
- User dismisses group → all notifications in group are dismissed
- Single notification from an app → no group (shows normally)

**Technical Notes:**
- Group key: `app_name + category` (from `hints.category`)
- Group struct:
  ```rust
  struct NotificationGroup {
      app_name: String,
      app_icon: String,
      notifications: Vec<Notification>,
      last_updated: Instant,
  }
  ```
- Max group size: 10 items (oldest removed when exceeded)

**Priority:** P1

#### US-4.4.2 — Group Expanded View
**As an** end user
**I want** to expand a notification group to see individual notifications
**so that** I can read and act on each one separately.

**Acceptance Criteria:**
- Expanding a group shows a stacked list of individual notifications
- Each item shows: icon + summary + timestamp
- Clicking an individual item opens its detail view (with actions)
- Swipe to dismiss works on individual items within the group
- Group shows "Clear All" button at bottom

**Edge Cases:**
- Group with 10+ items → scrollable list
- User dismisses all individual items → group disappears

**Technical Notes:**
- Component: `<NotificationGroupView>` renders a scrollable list
- Each item uses `<NotificationGroupItem>` with swipe-to-dismiss
- Dismiss IPC command includes both group_id and notification_id

**Priority:** P2

---

## Feature 4.5: Do Not Disturb Mode

**Description:** Implement a Do Not Disturb (DND) mode that suppresses non-critical notifications. Notifications are still received and queued but not displayed until DND is turned off.

**Acceptance Criteria:** DND toggle available. In DND: non-critical notifications queued but not shown. Critical notifications (alarms, calls) break through. DND can be time-scheduled.

**Dependencies:** 4.1, 4.2

**Notes:** Match Apple's Focus mode behavior. DND state stored in daemon and synced to UI.

### User Stories

#### US-4.5.1 — DND Toggle
**As an** end user
**I want** to toggle Do Not Disturb mode on and off
**so that** I can focus without notification interruptions.

**Acceptance Criteria:**
- DND toggle available in settings and context menu (long-press on pill)
- When DND is on:
  - Non-critical notifications are queued but not displayed
  - Pill shows a crescent moon icon (DND indicator)
  - Dismissed/expired notifications are not shown when DND turns off (use notification center instead)
- When DND is off: queued notifications are NOT retroactively shown (they were "missed")
- Critical notifications break through DND (per-app setting)

**Edge Cases:**
- Toggle during active notification → notification is dismissed (or kept, based on policy)
- DND automatically turns off at scheduled time → any notifications arriving during transition are handled normally

**Technical Notes:**
- DND state: `bool` stored in daemon config
- IPC event: `DndStatusChanged { enabled: bool }`
- Toggle via IPC command: `SetDnd { enabled: bool }`
- Visual: moon icon in pill when DND is active

**Priority:** P2

#### US-4.5.2 — Critical Notification Bypass
**As an** end user
**I want** critical notifications (alarms, calls) to bypass DND mode
**so that** I don't miss important alerts.

**Acceptance Criteria:**
- Notifications with `urgency=critical` or `category=alarm` or `category=incoming_call` bypass DND
- Bypassing notifications are displayed normally (pill shows them, sound plays)
- User can configure which notification categories bypass DND per-app

**Edge Cases:**
- Multiple critical notifications during DND → all are shown (not grouped with DND queue)
- App abuses critical urgency → user can block it in per-app settings

**Technical Notes:**
- Bypass detection: check `hints.urgency == 2` or `hints.category in ["alarm", "incoming_call", "timer"]`
- Per-app bypass stored in daemon config as `HashMap<String, Vec<Category>>`

**Priority:** P2

#### US-4.5.3 — Scheduled DND
**As an** end user
**I want** DND to automatically turn on and off on a schedule
**so that** I don't get notifications during focused work hours or sleep.

**Acceptance Criteria:**
- Schedule: start time + end time per day
- Days of week configuration (Mon–Sun)
- Multiple schedule entries supported (e.g., 9-12 work focus, 22-08 sleep)
- Schedule evaluated on daemon startup and at 1-minute intervals
- When schedule activates DND → notification in island "Do Not Disturb turned on"

**Edge Cases:**
- Overlapping schedules → last loaded wins
- Schedule change → apply immediately (no restart needed)
- User manually toggles DND while schedule is active → manual override takes priority until next schedule change

**Technical Notes:**
- Schedule stored in config:
  ```json
  {
    "dnd_schedule": [
      { "days": ["Mon","Tue","Wed","Thu","Fri"], "start": "22:00", "end": "08:00" }
    ]
  }
  ```
- Evaluation: tokio task runs every 60 seconds, checks current time against schedule
- Manual override flag: `manual_dnd_override: Option<bool>` — clears when schedule changes state

**Priority:** P3
