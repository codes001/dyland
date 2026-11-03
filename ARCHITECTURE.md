# Architecture Overview — dyland

**dyland** uses a decoupled, dual-process architecture designed for performance, isolation, and security on Linux desktops.

```
                  +-----------------------------------+
                  |            GNOME Desktop          |
                  +-----------------+-----------------+
                                    | D-Bus Signals
                                    v
+-------------------------------------------------------------------+
|                            dyd-daemon                             |
|  (Rust Core Service — Lifecycle, D-Bus Listeners, WASM Runtime)   |
|                                                                   |
|  +--------------------+  +------------------+  +---------------+  |
|  | D-Bus Sources      |  | Activity Queue   |  | ConfigStore   |  |
|  | (MPRIS, Notifs...) |  | (Priority Heap)  |  | (config.json) |  |
|  +---------+----------+  +--------+---------+  +-------+-------+  |
|            |                      |                    |          |
|            +----------------------+--------------------+          |
|                                   v                               |
|                        IPC Server (WebSocket)                     |
+-----------------------------------|-------------------------------+
                                    | JSON-RPC Envelopes
                                    v
+-------------------------------------------------------------------+
|                              dyd-ui                               |
|   (Tauri Transparent Overlay Window — React, Motion, Zustand)     |
|                                                                   |
|  +--------------------+  +------------------+  +---------------+  |
|  | Pill / State Engine|  | Expanded Views   |  | Theme System  |  |
|  +--------------------+  +------------------+  +---------------+  |
+-------------------------------------------------------------------+
```

---

## 🏛️ Components

### 1. `dyd-daemon` (Backend Core)
- Written in **Rust**.
- Runs as a systemd user service (`dyd-daemon.service`).
- Connects to D-Bus session and system buses via `zbus` for MPRIS, UPower, BlueZ, NetworkManager, and Notifications.
- Houses a WASM runtime (`wasmtime`) for sandboxed third-party plugins (`dyd-plugin-sdk`).
- Persists configuration to `$XDG_CONFIG_HOME/dyd/config.json` via atomic writes.

### 2. `dyd-ui` (Frontend Overlay)
- Built with **Tauri**, **React**, **Framer Motion**, and **Zustand**.
- Transparent, frameless utility window. On GNOME it is stacked above the top bar by the `dyland@dyland.app` shell extension, because Mutter does not implement layer-shell.
- Receives events over the daemon socket and renders the island. Input is limited to the island surface.

### 2b. `dyd-shell` (GNOME overlay integration)
- Hides the top-bar clock while the island is mapped.
- Reparents the dyland surface into the shell UI group and pins it to the top center of the primary monitor.
- Restores the clock when the extension is disabled, the screen locks, or the island process exits.

Login and crash recovery are specified in [docs/desktop-integration.md](docs/desktop-integration.md).

### 3. `dyd-types` (Shared Type Contract)
- Defines Rust structures (`IpcEvent`, `IpcCommand`, `AppConfig`, `ActivityPayload`) and TypeScript bindings (`types/src/index.ts`).
