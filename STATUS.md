# dyland — Implementation Status

> **Status flow:** `backlog` → `ready-for-dev` → `in-progress` → `review` → `done`

---

## Global Summary

| Epic | Status | Stories |
|------|--------|---------|
| **1. Foundation & Core Infrastructure** | **review** | 6/6 stories implemented (awaiting review) |
| **2. Pill — Rendering & Interaction** | **review** | 6/6 stories implemented (awaiting review) |
| **3. Media Integration** | **review** | 5/5 stories implemented (awaiting review) |
| **4. Notification Interception** | **done** | 2/2 stories implemented (4.3–4.5 deferred) |
| **5. System Status Indicators** | **done** | 6/6 stories implemented |
| **6. Plugin System** | **review** | 6/6 stories implemented (awaiting review) |
| **7. Settings & Preferences** | **backlog** | Not started |
| **8. Polish & Performance** | **backlog** | Not started |
| **9. Distribution** | **backlog** | Not started |

**Overall:** 31 stories implemented across Epics 1–6. Epics 1–3 & 6 in review; Epics 4–5 done. Epic 7 (Settings & Preferences) is next.

---

## Epic 1 — Foundation & Core Infrastructure

| Story | Status | What |
|-------|--------|------|
| 1.1 | **review** | Project Scaffolding & Tooling |
| 1.2 | **review** | Daemon Core — Lifecycle & D-Bus |
| 1.3 | **review** | UI Shell — Transparent Overlay Window |
| 1.4 | **review** | IPC Bridge — Daemon ↔ UI Communication |
| 1.5 | **review** | Pill Placeholder |
| 1.6 | **review** | Session Management — Auto-Start & Single Instance |

All tasks within each story are checked complete. Awaiting code review to move to `done`.

---

## Epic 2 — The Pill: Rendering & Interaction

| Story | Status | What |
|-------|--------|------|
| 2.1 | **review** | State Machine — Zustand store, transition logic, idle timeout, hidden state |
| 2.2 | **review** | Smooth Animations — motion.div spring animations, content transitions, blur backdrop |
| 2.3 | **review** | Gesture Handling — click/swipe/long-press with conflict resolution via pointer events |
| 2.4 | **review** | Activity Queue — daemon BinaryHeap queue + UI Zustand store + IPC wiring |
| 2.5 | **review** | Hover & Focus — CSS hover glow, :focus-visible keyboard outline, hover on expanded content |
| 2.6 | **review** | Fullscreen Detection — GNOME D-Bus polling, hidden state, monitor-aware, config override |

---

## Epic 3 — Media Integration

| Story | Status | What |
|-------|--------|------|
| 3.1 | **review** | MPRIS Source — Daemon-side MPRIS player detection and polling via D-Bus |
| 3.2 | **review** | Media Info Display — UI media store, MediaChanged handling, NowPlaying display |
| 3.3 | **review** | Media Playback Controls — MPRIS command forwarding, control buttons UI |
| 3.4 | **review** | Album Art — Fetch and display album art in ExpandedView with caching |
| 3.5 | **review** | Multi-Player Handling — Support multiple MPRIS players with priority/cycling |

---

## Epic 4 — Notification Interception

| Story | Status | What |
|-------|--------|------|
| 4.1 | **done** | D-Bus Notification Monitor |
| 4.2 | **done** | Notification Rendering in Island |
| 4.3 | **deferred** | Notification Actions — not implemented (post-v1) |
| 4.4 | **deferred** | Notification Grouping — not implemented (post-v1) |
| 4.5 | **deferred** | Do Not Disturb Mode — not implemented (post-v1) |

Epic scoped to the monitor + rendering pair for v1; 4.3–4.5 deferred.

---

## Epic 5 — System Status Indicators

| Story | Status | What |
|-------|--------|------|
| 5.1 | **done** | Battery Monitoring |
| 5.2 | **done** | WiFi Status |
| 5.3 | **done** | Bluetooth Status |
| 5.4 | **done** | Volume Control |
| 5.5 | **done** | Brightness Control |
| 5.6 | **done** | Status Icon Row |

---

## Epic 6 — Plugin System

| Story | Status | What |
|-------|--------|------|
| 6.1 | **review** | Plugin Manifest |
| 6.2 | **review** | Plugin Lifecycle & Registry |
| 6.3 | **review** | WASM Runtime — wasmtime sandbox + host API |
| 6.4 | **review** | Plugin SDK + `cargo-dyd` tooling |
| 6.5 | **review** | Plugin Management UI |
| 6.6 | **review** | Example Plugins (docker-status, vscode-git-status, terminal-notifier) |

---

## Epic 7 — Settings & Preferences

| Story | Status | What |
|-------|--------|------|
| 7.1 | **backlog** | Not started |
| 7.2 | **backlog** | Not started |
| 7.3 | **backlog** | Not started |
| 7.4 | **backlog** | Not started |
| 7.5 | **backlog** | Not started |

---

## Epic 8 — Polish & Performance

| Story | Status | What |
|-------|--------|------|
| 8.1 | **backlog** | Not started |
| 8.2 | **backlog** | Not started |
| 8.3 | **backlog** | Not started |
| 8.4 | **backlog** | Not started |
| 8.5 | **backlog** | Not started |

---

## Epic 9 — Distribution

| Story | Status | What |
|-------|--------|------|
| 9.1 | **backlog** | Not started |
| 9.2 | **backlog** | Not started |
| 9.3 | **backlog** | Not started |
| 9.4 | **backlog** | Not started |
| 9.5 | **backlog** | Not started |

---

*Last updated: 2026-08-04*
*Source: `_bmad-output/implementation-artifacts/sprint-status.yaml`*
