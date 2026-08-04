# Story 6.2: Plugin Lifecycle Management

Status: review

## Story

As a system,
I want to discover, load, run, and monitor plugins through a defined lifecycle in the daemon,
so that installed plugins can contribute content safely and be enabled/disabled without a daemon restart.

## Acceptance Criteria

1. Plugin directory scanned on daemon startup; each subdirectory contains a `manifest.json` and a WASM binary (US-6.2.1)
2. Plugin directory is created if missing; `DYD_PLUGINS_DIR` env var overrides, then `$XDG_DATA_HOME/dyd/plugins/`, then bundled plugins (US-6.2.1)
3. Manifests are validated; invalid ones are skipped with a warning (US-6.2.1)
4. Loaded plugins tracked in a `PluginRegistry` (HashMap<String, PluginHandle>); duplicate names are rejected (US-6.2.1)
5. Plugins can be loaded dynamically (without daemon restart) via IPC (US-6.2.1)
6. Lifecycle states: `Loaded` → `Initialized` → `Running` → `Stopped` → `Unloaded`, plus `Error` (US-6.2.2)
7. `init()`, `start()`, `stop()`, `unload()` invoked in order with state transitions logged (US-6.2.2)
8. `init()` failure → plugin stays in `Loaded`, error logged, not started (US-6.2.2)
9. `start()` timeout >5s → force-stopped (US-6.2.2)
10. Plugin panic/WASM trap → plugin transitions to `Error` state (US-6.2.2)
11. Plugin can report errors via `log_error`/`log_warn`/`log_info`; errors stored in a circular buffer (last 100) (US-6.2.3)
12. Plugin health accessible via IPC: `GetPluginHealth { name }` (US-6.2.3)
13. Plugin crashing >3 times in a rolling 5-minute window is automatically disabled; disabled notification sent to UI (US-6.2.3)

## Tasks / Subtasks

- [x] Design plugin registry module in daemon (AC: 1, 4, 5)
  - [x] Create `dyd-daemon/src/plugins/mod.rs` with `PluginRegistry` keyed by name
  - [x] `PluginHandle` struct holding name, manifest, state, instance handle, error count, last error, uptime
  - [x] Registry methods: `register`, `get`, `remove`, `list`, `set_state`
  - [x] Reject registering a second plugin with the same name
- [x] Implement plugin directory discovery (AC: 1, 2, 3)
  - [x] Resolve plugin dir: `DYD_PLUGINS_DIR` → `$XDG_DATA_HOME/dyd/plugins/` → bundled
  - [x] Create dir if missing
  - [x] Scan subdirectories; read `manifest.json` in each; validate via story 6-1 validator; skip invalid with warning
  - [x] Follow symlinked plugin directories
- [x] Implement lifecycle state machine (AC: 6, 7, 8, 9)
  - [x] `PluginState` enum: Loaded, Initialized, Running, Stopped, Unloaded, Error
  - [x] `load(name)` → register + Loaded; `init(name)` → Initialized; `start(name)` → Running; `stop(name)` → Stopped; `unload(name)` → Unloaded
  - [x] Log every state transition
  - [x] On init failure keep Loaded and log error
  - [x] Enforce 5s timeout on start; on timeout force-stop
- [x] Implement plugin health & error tracking (AC: 11, 12, 13)
  - [x] Per-plugin circular buffer of last 100 log/error entries
  - [x] `PluginHealth { state, last_error, error_count, uptime, memory_usage }` struct
  - [x] Rolling window: >3 crashes in 5 minutes → auto-disable plugin (state → Error, disabled flag)
  - [x] Emit daemon event/notification when a plugin is auto-disabled
- [x] Add IPC surface (AC: 5, 12)
  - [x] Add plugin commands to `types/src/lib.rs`: `GetPlugins`, `GetPluginHealth { name }`, `InstallPlugin { path: String }`, `SetPluginEnabled { name, enabled }`, `UninstallPlugin { name }`, `SetPluginConfig { name, config }`
  - [x] Add plugin events: `PluginChanged(PluginHealthPayload)`, `PluginActivity(ActivityPayload)` when a plugin contributes content
  - [x] Handle plugin IPC in daemon connection.rs
- [x] Wire plugin registry into daemon startup (AC: 1)
  - [x] Initialize registry in `Daemon::new()`, scan plugin dir in `Daemon::run()`
- [x] Add unit tests (AC: 1, 3, 4, 6, 7, 8, 9, 13)
  - [x] Test registry registers and rejects duplicate names
  - [x] Test discovery scans a temp dir, skips invalid manifests
  - [x] Test lifecycle transitions follow Loaded→Initialized→Running→Stopped→Unloaded
  - [x] Test init failure leaves state Loaded
  - [x] Test auto-disable after >3 crashes in rolling window
  - [x] Test error circular buffer keeps last 100

## Dev Notes

- Feature 6.2 in `bmad-output/EPIC-06-PLUGIN-SYSTEM.md`. This story covers the registry + lifecycle, but NOT the wasmtime sandbox execution — the host execution engine is the WASM story (6-3). Lifecycle methods here operate on a host-function-based execution stub that the 6-3 story replaces.
- Plugin directory notes: `$XDG_DATA_HOME/dyd/plugins/{name}/` (epic line 123). Resolve `$XDG_DATA_HOME` from env, fallback to `~/.local/share`.
- IPC conventions: `IpcCommand`/`IpcEvent` enums in `types/src/lib.rs`; handled in `dyd-daemon/src/ipc/connection.rs`; UI dispatch in `dyd-ui/src/ipc/client.ts`.
- The activity queue is `dyd-daemon/src/queue/` (`Activity`, `ActivityQueue`); plugins push content via the queue in the 6-3 story. This story only defines the plumbing.
- Sources register through `SourceContext` in `ipc/context.rs`; the plugin registry broadcasts via a `broadcast::Sender<IpcEnvelope>` just like media/notification sources.

### Project Structure Notes

- New file: `dyd-daemon/src/plugins/mod.rs` (registry + lifecycle + health)
- New file: `dyd-daemon/src/plugins/manager.rs` (directory discovery)
- Modify: `types/src/lib.rs` (plugin commands/events/payloads)
- Modify: `dyd-daemon/src/lib.rs` (registry init + startup scan)
- Modify: `dyd-daemon/src/ipc/context.rs`, `dyd-daemon/src/ipc/connection.rs` (plugin IPC)

### References

- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#Feature-62-Plugin-Lifecycle-Management]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-621-Plugin-Loading]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-622-Plugin-Lifecycle]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-623-Plugin-Health-Error-Reporting]
- [Source: existing dyd-daemon/src/sources/media.rs for broadcast pattern]
- [Source: existing dyd-daemon/src/ipc/connection.rs for command handling]

## Change Log

- 2026-08-03: Initial story creation
- 2026-08-03: Implemented plugin registry, directory discovery, lifecycle state machine, health/error tracking, and IPC surface.

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Created `dyd-daemon/src/plugins/` module: `mod.rs`, `handle.rs`, `host.rs`, `manager.rs`.
- `PluginHandle` tracks name, manifest, state, error_count, crash_times, 100-entry circular log buffer, config, is_core, disabled, memory_usage.
- `PluginState` enum: Loaded/Initialized/Running/Stopped/Unloaded/Error with as_str(). Lifecycle transitions logged via tracing.
- `PluginManager`: registry keyed by name (duplicates rejected), directory discovery (`DYD_PLUGINS_DIR` → `$XDG_DATA_HOME/dyd/plugins/` → HOME fallback), install/uninstall, set_enabled/set_config, auto-disable after >3 crashes in rolling 5-min window, emit_activity broadcast.
- `PluginExecutor` trait decouples lifecycle from sandbox; `StubExecutor` shipped for this story (no-op), replaced by wasmtime runtime in story 6-3.
- `PluginError` custom error type (Display + std::error::Error, no thiserror dep to avoid new deps).
- IPC: added `GetPlugins`, `GetPluginHealth`, `InstallPlugin`, `SetPluginEnabled`, `UninstallPlugin`, `SetPluginConfig` commands + `PluginChanged`/`PluginActivity` events in `types/src/lib.rs`; handlers in `ipc/connection.rs`; `SourceContext` carries `plugin_manager` + `plugin_broadcast`.
- Daemon wires registry in `Daemon::new()` and scans/starts plugins in `init_plugins()` during `run()`.
- `run_with_timeout` uses tokio spawn_blocking + 5s timeout (clippy disallows std::thread::spawn).
- Note: `GetPlugins` returns plugin list serialized inside a ConnectionStatus status string to avoid new reply-channel plumbing — flagged for review.
- 13 plugin unit tests pass; full daemon suite (43 tests) green; clippy clean (2 pre-existing warnings in notifications.rs remain).

### File List

- dyd-daemon/src/plugins/mod.rs (new)
- dyd-daemon/src/plugins/handle.rs (new)
- dyd-daemon/src/plugins/host.rs (new)
- dyd-daemon/src/plugins/manager.rs (new)
- dyd-daemon/src/lib.rs (modified: plugins module, plugin_manager field, init_plugins)
- dyd-daemon/src/ipc/context.rs (modified: plugin_manager, plugin_broadcast)
- dyd-daemon/src/ipc/connection.rs (modified: plugin command handlers)
- types/src/lib.rs (modified: plugin commands/events/payloads)