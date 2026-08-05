# Error Handling & Graceful Degradation Matrix — dyland

This document catalogs error states, recovery strategies, and graceful degradation behavior across all **dyland** subsystems.

---

## 1. Subsystem Error Matrix

| Subsystem | Error Condition | Expected Handling | Fallback / User Experience |
|---|---|---|---|
| **D-Bus Session Bus** | Connection fails at startup | Log warning, set `dbus_conn = None` | Dynamic Island launches cleanly; D-Bus features (MPRIS, notifications) enter degraded dormant state without crashing. |
| **D-Bus System Bus** | Connection fails at startup | Log warning, set `dbus_system_conn = None` | System status features (UPower, BlueZ, NetworkManager) degrade gracefully while media & UI continue operating. |
| **Battery Monitoring** | UPower D-Bus service missing or no battery present | Catch missing proxy error, disable battery listener | Battery indicator hidden from status row; app remains fully operational. |
| **Wi-Fi / NetworkManager** | NetworkManager service missing | Catch missing D-Bus interface, set state to disconnected | Wi-Fi status indicator hidden or displays disconnected state. |
| **Bluetooth / BlueZ** | BlueZ service unavailable or Bluetooth hardware missing | Handle missing D-Bus signal, set `present = false` | Bluetooth status indicator hidden; toggles disabled. |
| **Volume Control** | `wpctl` / `alsa` / `pactl` subprocess failure | Return volume `0.0` or clamp NaN/Inf values safely | Prevents daemon panic; UI displays silent volume state. |
| **Brightness Control** | `brightnessctl` or sysfs unreadable | Fallback to sysfs `/sys/class/backlight` polling | Brightness control degrades gracefully or hides slider. |
| **Config Persistence** | `config.json` corrupt or invalid JSON | Deep-merge over `AppConfig::default()`, log warning | Restores valid working configuration without losing user data or corrupting state. |
| **IPC WebSocket / Unix Socket** | Client disconnect or connection dropped | Reset client count, clean up IPC listeners | Automatic reconnect logic in UI (`IpcClient` exponential backoff); daemon continues running. |
| **WASM Plugin Runtime** | Plugin panics or memory overflow | Catch WASM trap, set plugin health `state = "error"`, isolate panic | Single plugin disabled; host daemon and all other plugins continue unaffected. |

---

## 2. Recovery & Health System

1. **Independent Module Health**:
   Every daemon subsystem operates inside an independent `tokio::spawn` worker task. Failure in one task does not propagate to others.
2. **Reconnection & Convergence**:
   D-Bus signal listeners auto-reconnect on service restart. Dynamic Island state automatically converges on the next emitted D-Bus event.
3. **IPC Resiliency**:
   IPC messages are structured as JSON-RPC envelopes with unique UUIDs. Malformed envelopes are logged and safely discarded.
