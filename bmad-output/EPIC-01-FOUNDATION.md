# Epic 1: Foundation & Core Infrastructure

## Objective
Establish the project structure, build tooling, daemon lifecycle, Tauri UI shell, IPC bridge, and Wayland transparent overlay window. After this Epic, developers can run the app and see a blank pill on screen.

## Business Value
Every subsequent Epic depends on this foundation. Getting this right prevents architectural debt across the entire project.

## Success Criteria
- `dyd-daemon` starts, connects to D-Bus session bus, stays alive
- `dyd-ui` launches as borderless, transparent, always-on-top window
- Window does not steal focus or interfere with input when dormant
- IPC communication works bidirectionally
- Window position is fixed at top-center of primary monitor
- Single-command dev workflow works
- Static gray pill visible on screen

## Risks
- GNOME 46 Mutter may not respect layer-shell consistently
- Wayland input passthrough requires `zwp_input_region_v1` — different API levels across compositors
- Tauri v2 Wayland support still maturing
- Auto-start requires systemd user service

## Dependencies
- Rust 1.80+ with tokio, zbus, serde, tracing
- Node 20+ with Vite, React, TypeScript, Tailwind, Motion
- Tauri v2
- Wayland client libraries

## Complexity
⭐⭐⭐⭐ — High risk due to Wayland/Compositor variability

---

## Feature 1.1: Project Scaffolding & Tooling

**Description:** Initialize the monorepo with Cargo workspace for `dyd-daemon`, npm workspace for `dyd-ui`, and shared type definitions. Set up build tooling, linting, formatting, and single-command dev workflow.

**Acceptance Criteria:** `make dev` starts both daemon and UI. Hot-reload on frontend changes. Rust recompilation on backend changes. All tools formattable/lintable in CI.

**Dependencies:** None

**Notes:** Cargo workspaces for Rust, npm workspaces for frontend. Prefer Cargo xtask or NPM scripts over Makefiles.

### User Stories

#### US-1.1.1 — Monorepo Initialization
**As a** developer
**I want** the project initialized as a monorepo with Cargo and npm workspaces
**so that** the codebase structure is established from day one.

**Acceptance Criteria:**
- `cargo build` compiles `dyd-daemon` binary
- `npm install` resolves all frontend dependencies
- Workspace members are defined in both `Cargo.toml` (virtual manifest) and `package.json` (npm workspaces)
- Shared types directory exists for Rust/TypeScript type definitions

**Edge Cases:**
- Cross-platform path differences (Linux/macOS/Windows for future contributors)
- Different Rust toolchain versions

**Technical Notes:**
- Root `Cargo.toml` virtual workspace with members `["dyd-daemon"]`
- Root `package.json` with workspaces `["dyd-ui"]`
- Shared types in `types/` directory with both Rust (`types/src/lib.rs`) and TypeScript (`types/src/index.ts`) definitions

**Priority:** P0

#### US-1.1.2 — Dev Workflow
**As a** developer
**I want** a single command that starts both the daemon and UI with hot-reload
**so that** I don't need to manually start two processes.

**Acceptance Criteria:**
- Single command (e.g., `cargo xtask dev` or `npm run dev`) starts both processes
- Frontend changes trigger Vite hot-reload
- Rust changes trigger recompilation (daemon restarts)
- Output from both processes is visible in a single terminal (or split)

**Edge Cases:**
- Daemon fails to start → UI shows clear error
- UI starts before daemon is ready → UI retries connection
- Port/socket conflicts

**Technical Notes:**
- Use `concurrently` (npm) or `cargo-make` for parallel process management
- Daemon stdout piped to terminal with `[daemon]` prefix
- Vite output with `[ui]` prefix

**Priority:** P0

#### US-1.1.3 — Linting & Formatting
**As a** developer
**I want** linting and formatting configured for both Rust and TypeScript
**so that** code quality is enforced across the project.

**Acceptance Criteria:**
- `cargo clippy` passes with no warnings (or agreed-upon allow list)
- `cargo fmt` passes
- `eslint` passes with project-specific ruleset
- `prettier` passes for TypeScript/Tailwind/CSS/JSON
- Pre-commit hooks (optional, configurable) run lint checks

**Edge Cases:**
- Rust nightly features required for some clippy lints

**Technical Notes:**
- `clippy.toml` for project-specific lint configuration
- ESLint flat config (`eslint.config.js`) for TypeScript + React
- `.prettierrc` with Tailwind plugin for class sorting

**Priority:** P1

#### US-1.1.4 — CI Configuration
**As a** developer
**I want** CI configuration that runs checks on every PR
**so that** regressions are caught before merging.

**Acceptance Criteria:**
- GitHub Actions workflow triggers on PR and push to main
- Workflow runs: `cargo build`, `cargo clippy`, `cargo fmt --check`, `cargo test`
- Workflow runs: `npm ci`, `npm run lint`, `npm run typecheck`, `npm run build`
- Workflow completes in under 5 minutes
- Status badge in README

**Edge Cases:**
- CI should not fail on formatting suggestions (should fail only on errors)
- Dependency caching for faster runs

**Technical Notes:**
- Use `Swatinem/rust-cache` for Rust compilation caching
- Use `actions/cache` for npm dependencies

**Priority:** P2

---

## Feature 1.2: Daemon Core — Lifecycle & D-Bus

**Description:** Implement `dyd-daemon` binary with tokio event loop, graceful shutdown, D-Bus session bus connection, and health-check IPC endpoint.

**Acceptance Criteria:** Daemon starts, connects to D-Bus, logs lifecycle events, handles SIGTERM/SIGINT cleanly, exposes health status.

**Dependencies:** 1.1

**Notes:** Use `zbus` for D-Bus, `tracing` for structured logging, `tokio` for async runtime.

### User Stories

#### US-1.2.1 — Daemon Entry Point & Event Loop
**As a** system
**I want** the daemon to start with a tokio runtime and connect to the D-Bus session bus
**so that** it can listen for system events.

**Acceptance Criteria:**
- `dyd-daemon` binary starts without panicking
- Tokio multi-threaded runtime initializes
- Connection to `org.freedesktop.DBus` session bus established
- Logs "daemon ready" with PID and D-Bus unique name
- Daemon stays alive until explicitly stopped

**Edge Cases:**
- `DBUS_SESSION_BUS_ADDRESS` not set → log helpful error and fall back
- No D-Bus session bus running → daemon should warn and retry, not crash

**Technical Notes:**
- Use `#[tokio::main]` on main function
- Use `zbus::Connection::session()` for connection
- Store connection handle for future signal subscriptions
- Grace period: wait up to 5 seconds for D-Bus, then exit with clear message

**Priority:** P0

#### US-1.2.2 — Graceful Shutdown
**As a** system
**I want** the daemon to handle SIGTERM and SIGINT gracefully
**so that** it can clean up resources before exiting.

**Acceptance Criteria:**
- SIGTERM received → flush pending logs → disconnect D-Bus → exit code 0
- SIGINT received → same cleanup sequence
- Ctrl+C in terminal triggers SIGINT path
- Cleanup completes within 500ms
- Double SIGTERM (force kill after grace period) → exit immediately

**Edge Cases:**
- SIGKILL cannot be caught — acceptable, state should be recoverable
- Shutdown during IPC message send → socket cleanup on next start

**Technical Notes:**
- Use `tokio::signal::unix` for signal handling
- Create a `Shutdown` token that propagates through the task graph
- Use `Drop` impls on key resources (D-Bus connection, socket listeners)

**Priority:** P0

#### US-1.2.3 — Health Check Endpoint
**As a** system
**I want** the daemon to expose a health status over IPC
**so that** the UI and external tools can verify connectivity.

**Acceptance Criteria:**
- IPC command `ping` → response `{ "status": "ok", "uptime_secs": 123, "version": "0.1.0" }`
- Health check does not block or wait on other subsystems
- Response includes: status, uptime, version, connected clients count

**Edge Cases:**
- Health check called before D-Bus connection established → status "degraded" with details
- Health check during shutdown → status "shutting_down"

**Technical Notes:**
- Implement as a match arm in the IPC message dispatcher
- Uptime tracked via `tokio::time::Instant::now()` at startup

**Priority:** P1

#### US-1.2.4 — Structured Logging
**As a** developer
**I want** structured logging with configurable output
**so that** I can debug issues in development and production.

**Acceptance Criteria:**
- Logs include: timestamp (ISO 8601), log level, target module, message, optional fields
- Default output: human-readable to stderr
- Configurable output: JSON to file or journald
- Log level configurable via env var `DYD_LOG=[trace|debug|info|warn|error]`
- File logging includes rotation (max 10MB, 5 files)

**Edge Cases:**
- Journald not available → fall back to file stderr
- Log file unwritable → fall back to stderr, emit warning once

**Technical Notes:**
- `tracing` crate for structured logging
- `tracing-subscriber` with `EnvFilter` for level control
- `tracing-journald` for journald integration (optional feature flag)
- Non-blocking file writer using `tracing-appender`

**Priority:** P2

---

## Feature 1.3: UI Shell — Transparent Overlay Window

**Description:** Create the Tauri application that renders a transparent, always-on-top, borderless window positioned at top-center of the primary monitor. Window must not steal focus and must pass through input events when pill is dormant.

**Acceptance Criteria:** Window appears at top-center, no title bar/decorations, no focus steal, click-through reaches app beneath, primary monitor only initially.

**Dependencies:** 1.1

**Notes:** Highest-risk feature. Must test on GNOME 46 Mutter. Fallback strategies for Wayland input region.

### User Stories

#### US-1.3.1 — Transparent Borderless Window
**As an** end user
**I want** the application window to be transparent and borderless
**so that** only the Dynamic Island content is visible on screen.

**Acceptance Criteria:**
- Window has no title bar or window decorations
- Window background is fully transparent (`rgba(0,0,0,0)`)
- No visible window chrome, resize handles, or borders
- Window background shows desktop background cleanly with no artifacts

**Edge Cases:**
- Compositor ignores transparency → window shows black rectangle (fallback: set explicit dark pill area only)
- GPU compositing pipeline causes transparency artifacts → test on Mesa and NVidia

**Technical Notes:**
- Tauri `tauri.conf.json`: `"decorations": false`, `"transparent": true`
- CSS: `html, body { background: transparent; margin: 0; padding: 0; overflow: hidden; }`
- Requires `decorations: false` in Tauri config

**Priority:** P0

#### US-1.3.2 — Always on Top
**As an** end user
**I want** the window to always stay on top of other windows
**so that** the Dynamic Island is always visible.

**Acceptance Criteria:**
- Window is positioned above all other application windows at all times
- Window remains on top across workspace switches
- Window remains on top when other apps go fullscreen (exceptions: see fullscreen detection later)

**Edge Cases:**
- Some fullscreen applications (games, video players) may override always-on-top behavior
- GNOME's "window focus" model may behave differently in overview mode vs normal mode

**Technical Notes:**
- Tauri: `"always_on_top": true` in tauri.conf.json
- On Wayland, always-on-top is compositor-dependent; may need layer-shell protocol for reliable behavior
- Alternative: use `zwlr_layer_shell_v1` (wlroots layer shell) if available

**Priority:** P0

#### US-1.3.3 — No Focus Stealing
**As an** end user
**I want** the Dynamic Island window to not steal keyboard focus
**so that** I can continue typing without interruption.

**Acceptance Criteria:**
- On launch, focus remains on the previously active window
- Clicking the dormant pill does NOT give it keyboard focus
- Clicking any other window immediately gives that window focus
- When the pill expands (interactive mode), it may request focus for text input

**Edge Cases:**
- Focus should transfer back to previous app when pill collapses
- Accessibility tools may require focus for screen readers

**Technical Notes:**
- Tauri: `"focus": false` and `"skip_taskbar": true`
- Wayland: use `zwp_input_region_v1` to declare an empty input region when dormant
- When expanded, add interactive areas to the input region
- This is the most platform-specific behavior — test thoroughly on GNOME 46

**Priority:** P0

#### US-1.3.4 — Top-Center Positioning
**As an** end user
**I want** the Dynamic Island window to appear at the top-center of my primary monitor
**so that** it matches the iPhone Dynamic Island position.

**Acceptance Criteria:**
- Window is centered horizontally on the primary monitor
- Window top edge is at y=0 (flush with screen top edge)
- Window width is ~400px (enough for expanded content, pill is centered within)
- Window height is ~100px (pill sits at top, expansion space below)
- Positioning recalculates on display configuration change

**Edge Cases:**
- Multi-monitor: appears only on primary monitor (initially, configurable later)
- Monitor hotplug: primary display may change
- Very small displays (<800px wide): reduce pill size proportionally
- Wayland fractional scaling: position calculation must account for scale factor

**Technical Notes:**
- Tauri: use `window.set_position()` with `PhysicalPosition`
- Use `window.available_monitors()` to determine primary monitor geometry
- Position: `x = (monitor_width - window_width) / 2`, `y = 0`
- Subscribe to `on_window_event(WindowEvent::Moved)` for repositioning triggers

**Priority:** P1

---

## Feature 1.4: IPC Bridge — Daemon ↔ UI Communication

**Description:** Establish bidirectional IPC channel between `dyd-daemon` and `dyd-ui` using Unix domain socket. Daemon emits structured JSON events, UI sends commands and receives responses.

**Acceptance Criteria:** UI subscribes to daemon events. Daemon pushes events at any time. UI sends commands and receives responses. IPC survives daemon restart (UI reconnects).

**Dependencies:** 1.2, 1.3

**Notes:** Daemon is a separate process — use Unix domain socket in abstract namespace. Protocol: newline-delimited JSON (one JSON object per line).

### User Stories

#### US-1.4.1 — Unix Domain Socket Transport
**As a** system
**I want** daemon and UI to communicate over a Unix domain socket in the abstract namespace
**so that** messages are secure, low-latency, and do not require filesystem access.

**Acceptance Criteria:**
- Daemon listens on `@dyd-daemon` (abstract socket namespace) on startup
- UI connects to the socket and maintains a persistent connection
- Messages are sent as newline-delimited JSON frames
- Socket permissions restrict access to the current user only
- Connection is non-blocking on both sides

**Edge Cases:**
- Abstract socket namespace not available on all Linux configurations
- Socket path collision (previous daemon instance still bound)
- Connection reset by peer

**Technical Notes:**
- Rust: `tokio::net::UnixListener` with abstract socket path (prepend `\0` or use `@`)
- Rust: `tokio::net::UnixStream` for client side
- TypeScript: Tauri's `invoke` or a custom Rust command that bridges to the socket
- Alternative: use Tauri's built-in `Channel` API with a Tauri plugin for the daemon (simpler but couples UI to daemon lifecycle)

**Priority:** P0

#### US-1.4.2 — Typed Event Protocol
**As a** developer
**I want** a shared type system for all IPC messages between daemon and UI
**so that** message structure is validated at compile time on both sides.

**Acceptance Criteria:**
- Every message has a `type` field (string enum) and a `payload` field (structured data)
- Rust side: all message types are `#[derive(Serialize, Deserialize)]` structs
- TypeScript side: all message types are typed interfaces matching the Rust structs
- Deserialization errors are caught and logged, not panicked
- Unknown message types are gracefully ignored with a warning

**Edge Cases:**
- Version mismatch between daemon and UI (new fields added)
- Malformed JSON (invalid UTF-8, truncated messages)
- Message too large (prevent DoS: max 1MB per message)

**Technical Notes:**
- Protocol envelope:
  ```json
  { "type": "EventName", "payload": { ... }, "id": "uuid" }
  ```
- Use `serde_json::Deserializer::from_reader` with `read_until('\n')` for framing
- TypeScript: discriminated union on `type` field
- Shared types generator: consider `ts-rs` crate to auto-generate TypeScript from Rust types
- Message size limit enforced at the socket read level

**Priority:** P0

#### US-1.4.3 — Automatic Reconnection
**As a** system
**I want** the UI to detect daemon disconnection and automatically reconnect
**so that** the application recovers from daemon restarts without user intervention.

**Acceptance Criteria:**
- When daemon disconnects (crash/restart), UI detects within 1 second
- UI enters "disconnected" state (visual indicator)
- UI attempts reconnection with exponential backoff (100ms, 200ms, 400ms, ... 5s max)
- When daemon comes back online, UI reconnects and resubscribes to events
- No memory leak from failed connection attempts

**Edge Cases:**
- Daemon crashes mid-message → partial message in buffer is discarded
- Daemon stays down for extended period → backoff caps at 5s interval, continues indefinitely
- Network namespace changes (containerization)

**Technical Notes:**
- Connection state managed in React context or Zustand store
- Exponential backoff: `base_delay * 2^attempt` capped at 5s, with jitter (±25%)
- Heartbeat/ping every 10 seconds to detect silent disconnection
- All event subscriptions are re-established on reconnect

**Priority:** P1

#### US-1.4.4 — Daemon Log Forwarding (Dev Mode)
**As a** developer
**I want** daemon log output to be visible in my development terminal alongside UI logs
**so that** I can debug the full system without opening separate terminal windows.

**Acceptance Criteria:**
- Dev workflow starts both processes with output merged
- Each log line prefixed with `[daemon]` or `[ui]` for clarity
- Log level and source file are visible in daemon log lines
- Colorized output for different log levels

**Edge Cases:**
- Large log volume should not cause terminal lag

**Technical Notes:**
- Use `concurrently` npm package with `--prefix` and `--timestamp` options
- Or: `cargo xtask dev` spawns both processes, pipes stdout/stderr

**Priority:** P3

---

## Feature 1.5: Render the Pill Placeholder

**Description:** Render a small black rounded-rectangle pill at the top-center of the overlay window. This is the visual anchor for all subsequent features. For now: static, non-interactive, dark-themed.

**Acceptance Criteria:** Centered pill (~126×36px, matching Apple proportions) visible. Background transparent outside pill. Correct corner radius (~18px).

**Dependencies:** 1.3, 1.4

**Notes:** Use Tailwind for styling. Pill is a React component from the start. Accepts children for future extension.

### User Stories

#### US-1.5.1 — Static Pill Rendering
**As an** end user
**I want** to see a small black pill at the top-center of my screen
**so that** I know the Dynamic Island application is running.

**Acceptance Criteria:**
- Pill is exactly 126px wide × 36px tall
- Pill has fully rounded corners (18px radius or `rounded-full`)
- Pill background is `#000000` (black)
- Pill is centered horizontally in the window (and therefore on screen)
- Pill sits flush at the top edge of the screen (y=0)
- No border, shadow, or outline around the pill

**Edge Cases:**
- High DPI displays: pill should appear the same physical size (use logical pixels)
- Screen resolution detection: pill should not be wider than the screen

**Technical Notes:**
- React component: `<Pill />`
- Tailwind: `w-[126px] h-[9px]` (36px = 9 * 4), though consider using fixed pixel values for precision
- Better approach: use inline styles or CSS with exact pixel values to match Apple dimensions precisely
- Position: `fixed top-0 left-1/2 -translate-x-1/2`

**Priority:** P0

#### US-1.5.2 — Full Transparency Outside Pill
**As an** end user
**I want** the area around the pill to be completely transparent
**so that** only the pill itself is visible on my desktop.

**Acceptance Criteria:**
- Desktop background is fully visible through the window outside the pill area
- No white/black matte, no artifacts, no flickering around the pill
- Works on both light and dark desktop backgrounds
- No visible window edges or corners

**Edge Cases:**
- Compositor transparency issues with NVidia GPU
- Fractional scaling causing anti-aliasing artifacts at pill edges

**Technical Notes:**
- CSS: `background: transparent` on `html` and `body`, plus `#app`
- Ensure `<body>` and `<div id="root">` have no background colors
- Use `overflow: hidden` on `body` to prevent scrollbars
- CSS: `* { margin: 0; padding: 0; box-sizing: border-box; }`

**Priority:** P0

#### US-1.5.3 — Reusable Pill Component Architecture
**As a** developer
**I want** the pill to be a well-structured, reusable React component
**so that** it can be extended with animations, states, and content slots later.

**Acceptance Criteria:**
- `Pill` component accepts props: `state`, `children`, `className`, `onClick`, `style`
- Component uses `forwardRef` for animation library access
- Component has no built-in state (is controlled by parent)
- Component is pure (same props → same render)
- Component is unit-testable with React Testing Library

**Edge Cases:**
- Empty children → render minimal pill width
- Many children → pill should not overflow (future: scroll or stack)

**Technical Notes:**
- Component API:
  ```typescript
  interface PillProps {
    state: 'dormant' | 'active' | 'expanded'
    children?: React.ReactNode
    className?: string
    style?: React.CSSProperties
    onClick?: () => void
  }
  ```
- Use `React.forwardRef<HTMLDivElement, PillProps>` for animation library access
- Default width: 126px (dormant), expands with content (active/expanded)
- Maintain a pill width constant file for consistent dimensions

**Priority:** P1

#### US-1.5.4 — Display Configuration Change Handling
**As an** end user
**I want** the pill position to update when I change my display configuration
**so that** it stays centered on the primary monitor.

**Acceptance Criteria:**
- When monitor is plugged/unplugged, pill re-centers on primary monitor
- When primary display changes (in GNOME Settings), pill moves to new primary
- When resolution changes, pill position updates
- Transition is smooth (position change animated, not snapped)

**Edge Cases:**
- All monitors disconnected (lid closed on laptop) → daemon pauses, UI hides
- Monitor added while pill is in expanded state → maintain expanded state, re-center

**Technical Notes:**
- Tauri: listen to `tauri://window-config-change` or `tauri://scale-change`
- Poll monitor geometry on `WindowEvent::Resized` events
- Use `window.primary_monitor()` to get current primary geometry

**Priority:** P2

---

## Feature 1.6: Session Management — Auto-Start & Single Instance

**Description:** Ensure only one daemon instance runs. Provide systemd user service for auto-start. Handle daemon crash → UI detects and reconnects.

**Acceptance Criteria:** Multiple daemon launches → second exits. Systemd user unit installable. UI shows disconnected state when daemon is down.

**Dependencies:** 1.2, 1.4

**Notes:** Use pidfile + `flock` for single-instance. Systemd user unit at `~/.config/systemd/user/dyd-daemon.service`.

### User Stories

#### US-1.6.1 — Single Instance Enforcement
**As a** system
**I want** only one instance of the daemon to run at a time
**so that** there are no conflicting state managers or resource contention.

**Acceptance Criteria:**
- Second daemon launch detects first instance is running
- Second instance exits immediately with exit code 0 and log message
- Stale pidfile after crash does not prevent restart (pidfile is validated)
- Lock is released on clean shutdown

**Edge Cases:**
- Daemon killed with SIGKILL → pidfile is stale, lock file may persist
- Multiple users on same system → each user has their own daemon (per-user, not system-wide)

**Technical Notes:**
- Create pidfile at `$XDG_RUNTIME_DIR/dyd-daemon.pid` or `/tmp/dyd-daemon.pid`
- Use `fs2` crate or `file-lock` for `fcntl`-based advisory locking
- On start: try to acquire lock → if fail, read pid from pidfile, check if process exists, if yes → exit, if stale → overwrite
- Alternative: use `tokio::sync::oneshot` with a Unix socket check (attempt to connect → if succeeds, daemon is running)

**Priority:** P0

#### US-1.6.2 — Systemd User Service for Auto-Start
**As an** end user
**I want** the daemon to start automatically when I log in
**so that** I don't have to manually start it.

**Acceptance Criteria:**
- Systemd user service `dyd-daemon.service` is available
- Service starts daemon binary with correct environment
- Service restarts on failure (Restart=on-failure, RestartSec=5s)
- Service stops cleanly on logout
- Service logs are accessible via `journalctl --user -u dyd-daemon`

**Edge Cases:**
- Systemd not available (non-systemd distro) → document manual start procedure
- Binary path changes after update → service file should use absolute path or PATH
- D-Bus session bus not ready when daemon starts → daemon should retry

**Technical Notes:**
- Service file:
  ```
  [Unit]
  Description=dyland Daemon
  After=graphical-session.target

  [Service]
  ExecStart=/usr/lib/dyd/dyd-daemon
  Restart=on-failure
  RestartSec=5s

  [Install]
  WantedBy=default.target
  ```
- Install command: `systemctl --user enable --now dyd-daemon.service`
- Part of `make install` target

**Priority:** P1

#### US-1.6.3 — Disconnected State UI
**As an** end user
**I want** to see a visual indicator when the daemon is not connected
**so that** I know the Dynamic Island is not fully functional.

**Acceptance Criteria:**
- When daemon is down, pill shows a "disconnected" indicator (e.g., gray with a small X or dash)
- When daemon reconnects, indicator disappears and normal operation resumes
- Indicator does not obscure any content or interfere with interaction
- No error popups or intrusive notifications

**Edge Cases:**
- Daemon repeatedly crashing and restarting → UI shows connection cycling state
- UI starts before daemon → initial state is "disconnected" until daemon is ready

**Technical Notes:**
- Connection state stored in React context or Zustand store
- States: `connecting | connected | disconnected | reconnecting`
- Visual: slightly dimmed pill with a small indicator dot (red = disconnected, yellow = connecting, green = connected)
- `Pill` component receives connection status as a prop

**Priority:** P1

#### US-1.6.4 — Install/Uninstall Commands
**As a** developer
**I want** `make install` and `make uninstall` commands
**so that** I can manage the daemon service during development and deployment.

**Acceptance Criteria:**
- `make install`:
  - Copies daemon binary to `/usr/lib/dyd/dyd-daemon`
  - Installs systemd user service file
  - Enables and starts the service
  - Copies UI resources
- `make uninstall`:
  - Stops and disables the service
  - Removes binary, service file, and resources
- Commands check for required permissions and provide clear error messages

**Edge Cases:**
- Not running as root → some operations may fail with clear instructions
- Reinstall while daemon is running → gracefully restart daemon
- Uninstall while UI is open → UI detects daemon gone, shows disconnected state

**Technical Notes:**
- Makefile targets that invoke `cargo build --release` then `install` command
- Install paths following FHS: `/usr/lib/dyd/`, `/usr/share/dyd/`, `/etc/dyd/`
- Or use Cargo xtask for cross-platform compatibility

**Priority:** P2
