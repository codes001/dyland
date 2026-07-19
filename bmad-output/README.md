# dyland — Apple-Style Dynamic Island for Linux

Product planning documents for the dyland project.

## Architecture

```
┌──────────────────────────────────────────────────┐
│                    dyd-ui                         │
│         (Tauri + React + TypeScript)              │
│  ┌──────────────────────────────────────────────┐│
│  │        Dynamic Island Pill (Motion)          ││
│  │     ┌─────┐  ┌────────────┐  ┌──────┐      ││
│  │     │Pill │──│  Expanded  │──│  Menu│      ││
│  │     └─────┘  │   View     │  └──────┘      ││
│  │              └────────────┘                  ││
│  └──────────────────────────────────────────────┘│
│              ↕ IPC (Unix Domain Socket)          │
├──────────────────────────────────────────────────┤
│                    dyd-daemon                     │
│              (Rust + Tokio + zbus)               │
│  ┌──────┐ ┌────────┐ ┌────────┐ ┌─────────────┐│
│  │MPRIS │ │Notif.  │ │System  │ │Plugin Host  ││
│  │Listnr│ │Monitor │ │Status  │ │(WASM)       ││
│  └──────┘ └────────┘ └────────┘ └─────────────┘│
│              ↕ D-Bus / System                    │
├──────────────────────────────────────────────────┤
│              Linux / GNOME / Wayland             │
└──────────────────────────────────────────────────┘
```

## Epics

| # | Epic | Status | Features | Stories |
|---|---|---|---|---|
| 1 | Foundation & Core Infrastructure | Planned | 6 | 24 |
| 2 | The Pill — Rendering & Interaction | Planned | 6 | 24 |
| 3 | Media Integration | Planned | 5 | 20 |
| 4 | Notification Interception | Planned | 5 | 20 |
| 5 | System Status Indicators | Planned | 6 | 24 |
| 6 | Plugin System | Planned | 6 | 24 |
| 7 | Settings & Preferences | Planned | 5 | 20 |
| 8 | Polish & Performance | Planned | 5 | 20 |
| 9 | Distribution | Planned | 5 | 20 |
| **Total** | | | **49** | **196** |

## Conventions

- **Priority:** P0 = Critical (blocker), P1 = High, P2 = Medium, P3 = Low
- **User types:** End User, Developer (plugin author), System Admin
