# Story 6.3: WASM Runtime for Plugins

Status: review

## Story

As a system,
I want plugins to execute in a sandboxed wasmtime runtime with memory/CPU limits and a host function API,
so that third-party code cannot crash or compromise the daemon while still interacting with the Dynamic Island.

## Acceptance Criteria

1. WASM plugins execute in a wasmtime runtime configured per plugin instance (US-6.3.1)
2. Memory limit: 10MB per instance (US-6.3.1)
3. CPU limit: fuel metering (100k fuel units per daemon tick, ~10ms); consuming all fuel pauses the plugin, resumed on next tick (not killed) (US-6.3.1)
4. No host filesystem access by default; no network access by default unless declared in manifest permissions (US-6.3.1)
5. WASM traps (panic) are caught and reported as plugin errors; instance isolated from other instances (US-6.3.1)
6. Host API functions available as WASM imports: `log_info`, `log_error`, `emit_activity`, `send_command`, `get_config`, `set_config`, `on_event`, `http_request`, `dbus_call` (US-6.3.2)
7. Each host function checks manifest permissions before executing; functions without granted permission return a permission-denied error (US-6.3.2)
8. Malformed JSON passed to host functions returns an error, does not crash the plugin (US-6.3.2)
9. HTTP request timeout >10s returns a timeout error (US-6.3.2)
10. Plugin can register UI component definitions in its manifest (`component_type: "expanded"` / `"icon"`); supported types table/progress/badge/list/custom (US-6.3.3)
11. Plugin sends structured data; daemon converts to a UI component; malformed data shows an error placeholder; unrecognized component type is ignored (US-6.3.3)
12. No arbitrary JavaScript execution from plugins (US-6.3.3)

## Tasks / Subtasks

- [ ] Add wasmtime dependency to daemon (AC: 1)
  - [ ] Add `wasmtime = "47"` to `dyd-daemon/Cargo.toml`
  - [ ] Confirm daemon still builds
- [ ] Implement sandboxed runtime engine (AC: 1, 2, 3, 4, 5)
  - [ ] Create `dyd-daemon/src/plugins/runtime.rs` with `SandboxedPlugin` per-instance Store + Instance
  - [ ] Configure memory limit 10MB (10 pages) per instance
  - [ ] Configure fuel metering: 100k fuel units per daemon tick (~10ms); exhaust → pause, resume next tick
  - [ ] Keep default capability/network access disabled
  - [ ] Catch WASM traps → plugin error state; instances fully isolated
- [ ] Implement host function API via Linker (AC: 6, 7, 8, 9)
  - [ ] Register imports: log_info/log_error, emit_activity, send_command, get_config/set_config, on_event, http_request (10s timeout), dbus_call
  - [ ] Each function gates on manifest permissions; denied → permission error string returned
  - [ ] String/JSON ABI: plugin allocates linear memory, passes pointer+length
  - [ ] Malformed JSON → error return, plugin continues
- [ ] Implement plugin UI contribution flow (AC: 10, 11, 12)
  - [ ] Parse manifest `ui_components` into registered component definitions
  - [ ] `emit_activity` payload maps structured data → typed UI component (table/progress/badge/list/custom)
  - [ ] Unknown component type ignored; malformed data → error placeholder event
  - [ ] No JS evaluation path exists anywhere in the flow
- [ ] Wire runtime into lifecycle (AC: 5)
  - [ ] Lifecycle start/stop invokes WASM exports `init`/`start`/`stop`/`unload` through the sandbox
  - [ ] Trap during lifecycle → Error state (feeds 6-2 health auto-disable)
  - [ ] Plugin activity emission enqueues into the daemon activity queue and broadcasts to UI
- Add unit tests (AC: 2, 3, 5, 7, 8, 9)
  - [ ] Test a minimal WASM module (from a small test fixture or inline wat) instantiates and runs
  - [ ] Test trap → error reported
  - [ ] Test fuel exhaustion pauses the instance
  - [ ] Test permission-denied on a host function without manifest permission
  - [ ] Test malformed JSON to emit_activity returns an error without crashing
  - [ ] Test emit_activity pushes an Activity to the queue

## Dev Notes

- Feature 6.3 in `bmad-output/EPIC-06-PLUGIN-SYSTEM.md`. wasmtime is the most mature WASM runtime for Rust (epic line 211).
- Target triple: `wasm32-wasip1` (modern name for wasm32-wasi). Rust 1.97 supports it via `rustup target add wasm32-wasip1`.
- Fuel metering: `store.set_fuel(100_000)` then consume in a loop; on `Trap::OutOfFuel` pause and resume next tick (epic lines 228-231).
- Memory: `MemoryType::new(10_000, Some(10_000))` ≈ 10MB (epic line 236).
- Host functions pass strings through linear memory: plugin allocates, writes, passes pointer+length. Keep the ABI minimal: `(ptr, len)` pairs for strings, return an i32 status code with an out-pointer for result strings.
- Permission strings follow the manifest format e.g. `dbus:org.freedesktop.DBus`, `http:localhost:2375`, `unix_socket:/var/run/docker.sock` (epic lines 78-79). Parse the prefix to gate host functions.
- Keep this story focused: HTTP client and D-Bus helper behavior are thin host wrappers. Full example plugins exercising them land in story 6-6.
- WASI preview-1 features should be left disabled (no fs/net) unless a manifest permission grants them — for v1 grant nothing beyond the host API. Plugin filesystem needs are out of scope.

### Project Structure Notes

- New file: `dyd-daemon/src/plugins/runtime.rs`
- New file: `dyd-daemon/src/plugins/host.rs` (host function linker + permission gating)
- Modify: `dyd-daemon/src/plugins/mod.rs` (lifecycle invokes runtime)
- Modify: `dyd-daemon/Cargo.toml` (wasmtime dep)
- Modify: `types/src/lib.rs` (PluginUiComponent payload)

### References

- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#Feature-63-WASM-Runtime-for-Plugins]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-631-WASM-Runtime-Integration]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-632-Host-Function-API-Imports]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-633-Plugin-UI-Contribution]
- [Source: existing dyd-daemon/src/queue/activity.rs for Activity queue integration]

## Change Log

- 2026-08-03: Initial story creation
- 2026-08-04: Implemented WASM runtime (`runtime.rs`): per-plugin wasmtime Store/Instance, 10 MiB `StoreLimits` memory cap, fuel metering (100k tick / 5M lifecycle), trap classification, all nine `"dyd"` host imports with permission gating, `(ptr,len)` + out-buffer-grow ABI. Wired a shared `HostContext`, `PluginManager::from_parts`/`tick_all`, and a 100 ms tick task in the daemon. Replaced the `GetPlugins` `ConnectionStatus` hack with a typed `PluginList` event. Added 15 `wat`-based tests. Status → review.

## Dev Agent Record

### Agent Model Used

claude-opus-4-8

### Debug Log References

### Completion Notes List

- **Fuel/pause model is pragmatic, not continuation-based.** `OutOfFuel` on `tick` → `TickOutcome::Paused` (logged, instance kept, re-invoked next tick with fresh fuel); `OutOfFuel` on a lifecycle export → `Error`. wasmtime `Config::async_support` is intentionally **off**; the executor stays synchronous and runs on `spawn_blocking`, so `Store: Send` and the existing 5 s `run_with_timeout` path is untouched.
- **Memory cap corrected vs. the story text.** AC2 is enforced with `StoreLimitsBuilder::memory_size(10 * 1024 * 1024)`, **not** the story's `MemoryType::new(10_000, Some(10_000))` — that maximum is 10_000 × 64 KiB ≈ 640 MiB (64× too large). A regression test (`memory_cap_rejects_oversized_initial_memory`) instantiates a 12.8 MiB module and asserts it fails while a 6.4 MiB one succeeds.
- **`http_request`** runs the real `reqwest` client (10 s baked-in timeout, AC9) via `ctx.rt.block_on(...)` from the blocking thread. `default-features=false` + `rustls-tls` to avoid a system OpenSSL dependency.
- **`dbus_call`** is a permission-gated **stub** returning `-5` (unimplemented) for this story — denied (`-1`) without a `dbus:` grant. Full zbus wiring is deferred to 6-6 as noted in Dev Notes; example plugins degrade gracefully.
- **`send_command`** publishes the parsed `IpcCommand` on the shared broadcast bus (gated by any `api:` grant). Full command round-trip execution is deferred; the ABI + gate are in place.
- **AC10/11 (UI component mapping) is partially delivered.** `ui_components` from the manifest already flow into `PluginHealthPayload` (surfaced to the UI in 6-5), and `emit_activity` carries arbitrary structured JSON. The typed table/progress/badge/list/custom *renderers* live in the 6-5 UI story; there is no JS-eval path anywhere (AC12 holds — plugins are WASM only).
- **`on_event`** is a read-and-ignore stub for this story (guest-directed event delivery not yet wired).
- Status codes shared with the future SDK: `>=0` ok / `-1` denied / `-2` malformed / `-3` bad memory / `-4` host error / `-5` unimplemented / `-6` not found. Malformed JSON and OOB pointers return a status and never trap (AC8), covered by `malformed_json_returns_minus_two` and `out_of_bounds_pointer_returns_minus_three`.
- Tests: 15 new `wat`-based tests (isolation, memory cap, fuel-pause, tick-trap→crash, emit→broadcast, perm gates, config roundtrip + out-buffer grow, malformed/OOB, dbus stub, permission unit tests). Full suite: `dyd-daemon` 57 passed, `dyd-types` 8 passed. `cargo clippy` clean for new code (2 pre-existing warnings in `sources/notifications.rs` only).

### File List

- NEW `dyd-daemon/src/plugins/runtime.rs` — `WasmExecutor`, `SandboxedPlugin`, `StoreState`, `HostContext`, `PermSet`, host-fn linker, ABI helpers.
- NEW `dyd-daemon/src/plugins/runtime_tests.rs` — 15 `wat`-based tests (included into `runtime::tests`).
- MOD `dyd-daemon/src/plugins/host.rs` — added `TickOutcome` + default `tick` on the `PluginExecutor` trait; updated `HostApi` doc-trait `http_request`/`dbus_call` to single-JSON-blob form.
- MOD `dyd-daemon/src/plugins/manager.rs` — `from_parts`, `tick_all`, shared `HostContext`, config seed on register + write-through on `set_config`.
- MOD `dyd-daemon/src/plugins/mod.rs` — export `runtime` + `TickOutcome`/`HostContext`/`PermSet`/`WasmExecutor`.
- MOD `dyd-daemon/src/lib.rs` — build `HostContext`+`WasmExecutor` in `Daemon::new` (stub fallback), 100 ms plugin tick task.
- MOD `dyd-daemon/Cargo.toml` — `reqwest` (rustls, no default features).
- MOD `dyd-daemon/src/ipc/connection.rs` — `GetPlugins` now emits typed `PluginList`.
- MOD `types/src/lib.rs` + `types/src/index.ts` — `PluginListPayload` + `IpcEvent::PluginList`.