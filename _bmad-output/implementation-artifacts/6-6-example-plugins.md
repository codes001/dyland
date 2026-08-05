# Story 6.6: Example Plugins

Status: review

## Story

As an end user,
I want working reference plugins that demonstrate the plugin API,
so that third-party developers have templates to copy and users can see plugin value.

## Acceptance Criteria

1. Docker status plugin works: connects to Docker socket, shows container count (running/total) in pill, container list with name/status/uptime in expanded view, updates every 10 seconds, permissions `unix_socket:/var/run/docker.sock` (US-6.6.1)
2. Docker not running → "Docker not available" state; permission denied → error shown (US-6.6.1)
3. VS Code Git status plugin works: monitors VS Code window title via D-Bus `org.gnome.Shell.Introspect`, extracts repo name/branch/dirty state/commits ahead-behind, shows `[branch] + [dirty count]` in pill, updates on title change, permissions `dbus:org.gnome.Shell.Introspect` (US-6.6.2)
4. VS Code not running → no indicator; no Git repo → "No repo" (US-6.6.2)
5. Terminal command notification plugin works: listens for shell notification signals, shows command name + exit code + duration, green checkmark on exit 0, red X on non-zero, click focuses terminal window, permissions `dbus:org.freedesktop.Notifications` (US-6.6.3)
6. Each plugin is documented with source code (US-6.6.1/2/3)

## Tasks / Subtasks

- [x] Scaffold example plugin workspace (AC: 6)
  - [x] Create `plugins/examples/` with three plugin crates: `docker-status`, `vscode-git-status`, `terminal-notifier`
  - [x] Each crate depends on `dyd-plugin-sdk` and builds for `wasm32-wasip1`
  - [x] Each crate ships a `manifest.json` declaring permissions and ui_components
- [x] Implement docker-status plugin (AC: 1, 2)
  - [x] `init` connects to `unix:///var/run/docker.sock`
  - [x] Timer `every_10s` → read container list → emit running/total + container list activity
  - [x] Handle docker-unavailable and permission-denied states
  - [x] manifest: permissions `unix_socket:/var/run/docker.sock`, ui_components `["expanded"]`
- [x] Implement vscode-git-status plugin (AC: 3, 4)
  - [x] D-Bus introspection of VS Code window title via `org.gnome.Shell.Introspect`
  - [x] Parse repo name, branch, dirty state, commits ahead/behind from title
  - [x] Emit `[branch]` + `[dirty]` pill content on title change
  - [x] Handle VS Code-not-running (no indicator) and no-repo ("No repo") cases
  - [x] manifest: permissions `dbus:org.gnome.Shell.Introspect`, ui_components `["expanded"]`
- [x] Implement terminal-notifier plugin (AC: 5)
  - [x] Listen on `org.freedesktop.Notifications` for shell completion signals
  - [x] Extract command name, exit code, duration
  - [x] Emit activity with success/failure glyph (✓ / ✗)
  - [x] Provide focus-terminal action (click → focus terminal window)
  - [x] manifest: permissions `dbus:org.freedesktop.Notifications`, ui_components `["expanded"]`
- [x] Add README docs for each plugin (AC: 6)
  - [x] Document build (`cargo dyd build`), install, and usage for each plugin
- Add tests (AC: 1, 3, 5)
  - [x] Test docker-status parses a sample docker ps JSON into running/total + list
  - [x] Test vscode-git-status parses a sample window title into repo/branch/dirty fields
  - [x] Test terminal-notifier parses a sample completion payload into command/exit/duration
  - [x] Build all three plugins for `wasm32-wasip1` (compile check)

## Dev Notes

- Feature 6.6 in `bmad-output/EPIC-06-PLUGIN-SYSTEM.md`. These plugins exercise the SDK (6-4) and the daemon host API (6-3).
- Docker socket connection uses the host `http_request` capability over `unix_socket:` permission — daemon host-side maps the unix-socket permission into an HTTP-over-unix-socket client.
- D-Bus from WASM goes through the daemon `dbus_call` host function gated by `dbus:` permission strings.
- The example plugins are best-effort reference implementations. If the daemon host API (6-3) cannot fully support a plugin's exact scenario (e.g., unix socket HTTP), the plugin should degrade gracefully and the limitation documented in the plugin README + completion notes.
- Keep each plugin small (epic suggests ~120-200 lines): the goal is idiomatic templates.
- `.dyd` packaging is optional for this story (covered by 6-4 tooling); if `cargo dyd` is unavailable the plugins ship as source + manifest.

### Project Structure Notes

- New: `plugins/examples/docker-status/`
- New: `plugins/examples/vscode-git-status/`
- New: `plugins/examples/terminal-notifier/`
- Each: `Cargo.toml`, `src/lib.rs`, `manifest.json`, `README.md`

### References

- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#Feature-66-Example-Plugins]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-661-Docker-Status-Plugin]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-662-VS-Code-Git-Status-Plugin]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-663-Terminal-Command-Notification-Plugin]
- [Source: plugins/sdk (story 6-4)]

## Change Log

- 2026-08-03: Initial story creation
- 2026-08-04: Implemented all three reference plugins (docker-status, vscode-git-status, terminal-notifier) with parsing unit tests, manifests, and READMEs. Status → review.

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free
claude-opus-4-8

### Debug Log References

### Completion Notes List

- **Reference-quality + parse-tested (not live end-to-end).** Per the story Dev Notes, each plugin's parsing/formatting logic is fully unit-tested on the host (`cargo test`, 5 tests each = 15 total, all pass) and every crate compiles clean for `wasm32-wasip1` (docker 71KB, vscode-git-status 50KB, terminal-notifier 41KB — all well under the 500KB budget) with zero clippy warnings against the wasm target.
- **Host-support caveats (documented in each README).** Three host capabilities remain stubs from story 6-3, so the live scenarios degrade gracefully rather than run end-to-end:
  - `docker-status` needs HTTP-over-unix-socket; the host `http_request` is TCP-only today, so the plugin renders the "Docker not available" path until the unix-socket bridge lands. Container-JSON parsing is fully tested.
  - `vscode-git-status` needs `dbus_call`, which returns `-5 (unimplemented)`; the plugin renders nothing (VS Code-not-running path) until real D-Bus wiring lands. The window-title parser (incl. `↑N↓N` ahead/behind) is fully tested.
  - `terminal-notifier` needs inbound `on_event` delivery (not yet forwarded to plugins) and the `api:` capability for click-to-focus; the payload parser and activity formatting are fully tested and work unchanged once delivery lands.
- **Host-testable no_std pattern.** Each crate uses `#![cfg_attr(not(test), no_std)]` so `cargo test` links against std (test harness + allocator) while the wasm build stays no_std, resolving the allocator/panic_handler/mem-intrinsics from the SDK's wasm32-gated block.
- **clippy note.** `cargo clippy` on the host target reports "no global allocator / no panic_handler" because those items are wasm32-gated; clippy must be run with `--target wasm32-wasip1` (0 warnings) — the host `cargo test` build supplies them via std.
- **vscode-git-status parser fix.** The initial `parse_branch_segment` split on whitespace, so the adjacent `↑1↓2` marker (no space, the documented VS Code `window.title` format) failed to parse. Rewrote it to scan the segment for `↑`/`↓` and read the following digit run for each independently; branch name is the text preceding the first arrow marker.

### File List

- `plugins/examples/docker-status/Cargo.toml`
- `plugins/examples/docker-status/src/lib.rs`
- `plugins/examples/docker-status/manifest.json`
- `plugins/examples/docker-status/README.md`
- `plugins/examples/vscode-git-status/Cargo.toml`
- `plugins/examples/vscode-git-status/src/lib.rs`
- `plugins/examples/vscode-git-status/manifest.json`
- `plugins/examples/vscode-git-status/README.md`
- `plugins/examples/terminal-notifier/Cargo.toml`
- `plugins/examples/terminal-notifier/src/lib.rs`
- `plugins/examples/terminal-notifier/manifest.json`
- `plugins/examples/terminal-notifier/README.md`