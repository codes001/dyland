# Story 6.6: Example Plugins

Status: ready-for-dev

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

- [ ] Scaffold example plugin workspace (AC: 6)
  - [ ] Create `plugins/examples/` with three plugin crates: `docker-status`, `vscode-git-status`, `terminal-notifier`
  - [ ] Each crate depends on `dyd-plugin-sdk` and builds for `wasm32-wasip1`
  - [ ] Each crate ships a `manifest.json` declaring permissions and ui_components
- [ ] Implement docker-status plugin (AC: 1, 2)
  - [ ] `init` connects to `unix:///var/run/docker.sock`
  - [ ] Timer `every_10s` → read container list → emit running/total + container list activity
  - [ ] Handle docker-unavailable and permission-denied states
  - [ ] manifest: permissions `unix_socket:/var/run/docker.sock`, ui_components `["expanded"]`
- [ ] Implement vscode-git-status plugin (AC: 3, 4)
  - [ ] D-Bus introspection of VS Code window title via `org.gnome.Shell.Introspect`
  - [ ] Parse repo name, branch, dirty state, commits ahead/behind from title
  - [ ] Emit `[branch]` + `[dirty]` pill content on title change
  - [ ] Handle VS Code-not-running (no indicator) and no-repo ("No repo") cases
  - [ ] manifest: permissions `dbus:org.gnome.Shell.Introspect`, ui_components `["expanded"]`
- [ ] Implement terminal-notifier plugin (AC: 5)
  - [ ] Listen on `org.freedesktop.Notifications` for shell completion signals
  - [ ] Extract command name, exit code, duration
  - [ ] Emit activity with success/failure glyph (✓ / ✗)
  - [ ] Provide focus-terminal action (click → focus terminal window)
  - [ ] manifest: permissions `dbus:org.freedesktop.Notifications`, ui_components `["expanded"]`
- [ ] Add README docs for each plugin (AC: 6)
  - [ ] Document build (`cargo dyd build`), install, and usage for each plugin
- Add tests (AC: 1, 3, 5)
  - [ ] Test docker-status parses a sample docker ps JSON into running/total + list
  - [ ] Test vscode-git-status parses a sample window title into repo/branch/dirty fields
  - [ ] Test terminal-notifier parses a sample completion payload into command/exit/duration
  - [ ] Build all three plugins for `wasm32-wasip1` (compile check)

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

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List