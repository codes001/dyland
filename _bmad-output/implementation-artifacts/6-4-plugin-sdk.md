# Story 6.4: Plugin SDK & Tooling

Status: ready-for-dev

## Story

As a developer (plugin author),
I want an ergonomic Rust SDK and build tooling that make WASM plugin development straightforward,
so that I can focus on plugin logic instead of WASM glue code.

## Acceptance Criteria

1. SDK crate (`dyd-plugin-sdk`) exists and compiles for `wasm32-wasip1` target (US-6.4.1)
2. SDK provides `log_info!`, `log_error!` macros, `emit_activity!` macro, `subscribe_event!` macro (US-6.4.1)
3. SDK provides HTTP client helper `http::get()`, `http::post()` (US-6.4.1)
4. SDK provides D-Bus helper `dbus::call()` (US-6.4.1)
5. SDK provides config helpers `config::get()`, `config::set()` (US-6.4.1)
6. SDK is a `no_std` crate (WASM target may not have std); WASI preview-1 provides support (US-6.4.1)
7. Generated WASM binary <500KB for a minimal plugin (US-6.4.1)
8. SDK version is declared in manifest `dependencies`/`sdk_version` for compatibility (US-6.4.1)
9. Plugin build tooling: `cargo-dyd` CLI with `init`, `build`, `package`, `install` subcommands (US-6.4.2)
10. Plugin package format: `.dyd` zip containing `manifest.json`, `plugin.wasm`, optional `assets/` (US-6.4.2)
11. Install unzips to `$XDG_DATA_HOME/dyd/plugins/{name}/` (US-6.4.2)
12. Missing `wasm32-wasip1` target produces a clear error suggesting `rustup target add wasm32-wasip1` (US-6.4.2)

## Tasks / Subtasks

- [ ] Create `dyd-plugin-sdk` crate (AC: 1, 6)
  - [ ] Add workspace member `plugins/sdk` with crate name `dyd-plugin-sdk`
  - [ ] Configure `#![no_std]` with `extern crate alloc`; add `serde`/`serde_json` no_std deps
  - [ ] Provide `dyd_plugin_sdk::prelude::*`
  - [ ] Ensure it builds for `x86_64-unknown-linux-gnu` AND `wasm32-wasip1`
- [ ] Implement host ABI binding (AC: 2, 3, 4, 5)
  - [ ] Declare extern "C" imports matching daemon host functions (log_info, log_error, emit_activity, send_command, get_config, set_config, on_event, http_request, dbus_call)
  - [ ] Implement string/JSON marshalling helpers (alloc in linear memory, pass ptr+len, read results)
  - [ ] `log_info!`/`log_error!` macros
  - [ ] `emit_activity!` macro
  - [ ] `subscribe_event!` macro
  - [ ] `http::get()`/`http::post()` helpers
  - [ ] `dbus::call()` helper
  - [ ] `config::get()`/`config::set()` helpers
- [ ] Implement plugin entry macro / plugin template (AC: 2)
  - [ ] Provide `plugin_entry!` macro or explicit `#[no_mangle] pub extern "C" fn init/start/stop/unload` pattern
  - [ ] Provide a minimal plugin template crate for scaffolding
- [ ] Implement `cargo-dyd` CLI tool (AC: 9, 10, 11, 12)
  - [ ] Add workspace member `plugins/cargo-dyd` (binary)
  - [ ] `cargo dyd init` → scaffold a new plugin project (copy template + manifest.json)
  - [ ] `cargo dyd build` → run `cargo build --release --target wasm32-wasip1`; detect missing target and print `rustup target add wasm32-wasip1` guidance
  - [ ] `cargo dyd package` → create `{name}-{version}.dyd` zip (manifest.json + plugin.wasm + optional assets/)
  - [ ] `cargo dyd install` → unzip `.dyd` to `$XDG_DATA_HOME/dyd/plugins/{name}/`
- Add tests (AC: 7, 9, 10)
  - [ ] Build a minimal plugin against the SDK for `wasm32-wasip1` and assert the wasm artifact size <500KB
  - [ ] Unit test zip package layout (manifest.json + plugin.wasm present, assets preserved)
  - [ ] Unit test `cargo dyd init` scaffold contains manifest.json + src/lib.rs
  - [ ] Test missing-target error message contains the rustup hint

## Dev Notes

- Feature 6.4 in `bmad-output/EPIC-06-PLUGIN-SYSTEM.md`. Target triple `wasm32-wasip1` (formerly `wasm32-wasi`) for standard WASI support (epic line 344).
- The SDK uses extern C imports that match exactly the host functions the daemon registers (see story 6-3 host.rs). Keep the ABI in sync — it is the contract between SDK and daemon.
- no_std: `alloc` is available; serde_json works in no_std with the `alloc` feature. Add `serde_json = { version = "1", default-features = false, features = ["alloc"] }`.
- Macros are the priority; a full proc-macro attribute `#[plugin]` (epic US-6.4.1) is the ideal end state but can be delivered as a declarative macro `plugin_entry!` for this story to stay reviewable. Note this deviation in Completion Notes.
- CLI uses clap; zip creation via the `zip` crate.
- `cargo-dyd` should be installable via `cargo install --path plugins/cargo-dyd` or built as a workspace member binary.
- Story 6-6 (example plugins) depends on this SDK being usable end-to-end.

### Project Structure Notes

- New crate: `plugins/sdk` (workspace member, name `dyd-plugin-sdk`)
- New crate: `plugins/cargo-dyd` (workspace member, binary `cargo-dyd`)
- Modify: `Cargo.toml` workspace members
- The daemon is unaffected by this story.

### References

- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#Feature-64-Plugin-SDK-Tooling]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-641-Plugin-SDK-Crate]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-642-Plugin-Build-Tooling]

## Change Log

- 2026-08-03: Initial story creation

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List