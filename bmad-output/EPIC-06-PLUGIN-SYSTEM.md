# Epic 6: Plugin System

## Objective
Design and implement a plugin system that allows third-party developers to extend the Dynamic Island with custom content sources, interactive controls, and visual components. Plugins run in a sandboxed WASM runtime for safety.

## Business Value
An open plugin system transforms dyland from a fixed-feature app into a platform. The community can create integrations for Docker, Git, VS Code, CI/CD, weather, stock tickers, and more without waiting for core team implementation.

## Success Criteria
- Plugin manifest format defined and validated
- Plugins can be installed, enabled, disabled, and uninstalled without restart
- Plugin lifecycle (load, init, start, stop, unload) works correctly
- Plugin crashes do not crash the daemon
- Plugin API provides: D-Bus access, IPC channels, timer/scheduler, HTTP client, notification access
- Plugin can contribute content to the Dynamic Island activity queue
- Plugin can render custom UI in the expanded view (via injected HTML/CSS or pre-defined components)
- WASM runtime memory and CPU limits enforced
- At least one example plugin works end-to-end

## Risks
- WASM runtime in Rust (wasmtime) adds ~10MB to daemon binary size
- WASM plugin API design is difficult to get right — too restrictive stifles creativity, too permissive risks security
- Plugin UI rendering in a web context is complex (Tauri webview + custom component injection)
- Versioning and compatibility will be an ongoing maintenance burden
- Premature plugin system can slow down development of core features

## Dependencies
- Epic 1 (daemon, IPC all required)
- Epics 2-5 (plugin system needs the activity queue and event bus to be stable)

## Complexity
⭐⭐⭐⭐⭐ — Very high. Plugin API design, WASM runtime integration, sandboxing, and dynamic UI injection are all challenging.

---

## Feature 6.1: Plugin Manifest Format

**Description:** Define the plugin manifest format (JSON/YAML) that describes a plugin's metadata, permissions, entry points, and dependencies. Manifests are validated on installation.

**Acceptance Criteria:** Manifest schema defined and documented. Validation rejects invalid manifests with clear error messages. Manifests include: name, version, author, description, permissions, entry points, UI components.

**Dependencies:** None (design-only feature)

**Notes:** Follow npm/Cargo manifest conventions where appropriate. Keep it simple — avoid over-engineering the format.

### User Stories

#### US-6.1.1 — Manifest Schema Definition
**As a** developer (plugin author)
**I want** a clear, well-documented plugin manifest format
**so that** I can create plugins that the daemon can load.

**Acceptance Criteria:**
- Manifest format: JSON (or TOML for easier hand-editing)
- Required fields: `name`, `version`, `author`, `entry_point`
- Optional fields: `description`, `homepage`, `license`, `permissions`, `ui_components`, `dependencies`, `config_schema`
- Permissions array: list of D-Bus services, system resources, or API features the plugin needs
- Entry point: WASM binary path (relative to plugin directory)
- Schema version field for future compatibility

**Edge Cases:**
- Missing required field → validation error with field name
- Unknown permission → warning, ignore (forward-compatible)
- Version string not semver → warning, best-effort parse

**Technical Notes:**
- Schema definition in TypeScript (for UI validation) and Rust (for daemon validation)
- Use `serde` + `schemars` for Rust, `zod` for TypeScript
- Example manifest:
  ```json
  {
    "schema_version": 1,
    "name": "docker-status",
    "version": "0.1.0",
    "author": "dyland",
    "description": "Show Docker container status in Dynamic Island",
    "entry_point": "plugin.wasm",
    "permissions": ["dbus:org.freedesktop.DBus", "http:localhost:2375"],
    "ui_components": ["expanded"],
    "config_schema": {
      "type": "object",
      "properties": {
        "docker_host": { "type": "string", "default": "unix:///var/run/docker.sock" }
      }
    }
  }
  ```

**Priority:** P1

#### US-6.1.2 — Manifest Validation
**As a** system
**I want** plugin manifests to be validated before installation
**so that** malformed plugins do not compromise the daemon.

**Acceptance Criteria:**
- Validation checks: required fields, field types, permission names are recognized, version is parseable
- Validation errors are returned with field path and reason
- Warning for unknown fields (forward-compatible)
- Validation runs on install and on daemon startup (for installed plugins)
- Invalid plugins are skipped with error log

**Edge Cases:**
- Manifest is valid JSON but semantically wrong (bad URL, etc.) → semantic validation
- Permission that doesn't exist yet → allowed (forward-compatible, ignored)

**Technical Notes:**
- Rust: use `serde_json::from_str` + manual field validation
- TypeScript: `z.object({...}).strict().parse()`

**Priority:** P2

---

## Feature 6.2: Plugin Lifecycle Management

**Description:** Implement plugin loading, initialization, starting, stopping, and unloading. Plugins are loaded from a plugins directory. Lifecycle is managed by the daemon.

**Acceptance Criteria:** Plugins can be installed/copied to plugins directory. Daemon loads plugins on startup. Plugin lifecycle methods are called in order. Plugin can be enabled/disabled via IPC. Plugin crashes are handled gracefully.

**Dependencies:** 1.2, 6.1

**Notes:** Plugin directory: `$XDG_DATA_HOME/dyd/plugins/{name}/`. Lifecycle: load → validate → init → start → stop → unload.

### User Stories

#### US-6.2.1 — Plugin Loading
**As a** system
**I want** to discover and load plugins from the plugins directory
**so that** installed plugins are available for use.

**Acceptance Criteria:**
- Plugin directory scanned on daemon startup
- Each subdirectory contains a `manifest.json` and the WASM binary
- Manifests are validated, invalid ones are skipped with warning
- Loaded plugins are tracked in a `PluginRegistry`
- Plugins can be loaded dynamically (without daemon restart) via IPC command

**Edge Cases:**
- Plugin directory does not exist → create it
- Symlink in plugin directory → follow it
- Two plugins with same name → second one is rejected

**Technical Notes:**
- Plugin directory path: first from `DYD_PLUGINS_DIR` env var, then `$XDG_DATA_HOME/dyd/plugins/`, then bundled plugins
- Plugin registry: `HashMap<String, PluginHandle>`
- Dynamic load IPC: `InstallPlugin { path: String }` → daemon copies to plugins dir and loads

**Priority:** P1

#### US-6.2.2 — Plugin Lifecycle (Init → Start → Stop)
**As a** system
**I want** plugins to go through a defined lifecycle
**so that** their execution is predictable and safe.

**Acceptance Criteria:**
- Lifecycle states: `Loaded` → `Initialized` → `Running` → `Stopped` → `Unloaded`
- `init()`: called after load, plugin sets up its state, returns config defaults
- `start()`: plugin begins listening for events, may register activity sources
- `stop()`: plugin stops all activity, cleans up resources
- `unload()`: final cleanup, WASM instance is dropped
- State transitions are logged
- Plugin can report health status

**Edge Cases:**
- `init()` fails → plugin stays in `Loaded` state, error logged, not started
- `start()` times out (>5s) → plugin is force-stopped
- Plugin panics → WASM trap caught, plugin transitions to `Error` state

**Technical Notes:**
- Each lifecycle method is a WASM export function
- Timeout for each method: 5 seconds (enforced by wasmtime)
- Error state: plugin is disabled until manually re-enabled

**Priority:** P1

#### US-6.2.3 — Plugin Health & Error Reporting
**As a** system
**I want** to monitor plugin health and report errors
**so that** problematic plugins can be identified and disabled.

**Acceptance Criteria:**
- Plugin can report errors via API (`log_error`, `log_warn`, `log_info`)
- Plugin errors are stored in a circular buffer (last 100 entries)
- Plugin health status accessible via IPC: `GetPluginHealth { name }`
- Plugin that crashes >3 times in 5 minutes is automatically disabled
- Disabled plugin notification sent to UI

**Edge Cases:**
- Plugin enters infinite loop → WASM fuel metering kills it (see 6.3)
- Plugin consumes too much memory → WASM memory limit enforced

**Technical Notes:**
- Health struct: `{ state, last_error, error_count, uptime, memory_usage }`
- Auto-disable: `error_count > 3 in rolling 5min window`
- User can manually re-enable from settings

**Priority:** P2

---

## Feature 6.3: WASM Runtime for Plugins

**Description:** Integrate a WASM runtime (wasmtime) into the daemon to execute plugin code in a sandboxed environment with resource limits.

**Acceptance Criteria:** WASM plugins execute in wasmtime runtime. Memory limited to 10MB per plugin. CPU limited via fuel metering (100k fuel units per tick). No access to host filesystem or network unless explicitly permitted. Host functions available to plugins via import bindings.

**Dependencies:** 6.2

**Notes:** wasmtime is the most mature WASM runtime for Rust. Use its fuel metering for CPU limiting and memory limits for safety.

### User Stories

#### US-6.3.1 — WASM Runtime Integration
**As a** system
**I want** plugins to execute in a wasmtime sandbox
**so that** they cannot crash or compromise the daemon.

**Acceptance Criteria:**
- wasmtime runtime configured per plugin instance
- Memory limit: 10MB per instance
- CPU limit: fuel metering (1 unit = 1 instruction, 100k units per daemon tick, ~10ms)
- No host filesystem access by default
- No network access by default (must be declared in manifest permissions)
- WASM traps (panic) are caught and reported as plugin errors
- Plugin instance is isolated from other plugin instances

**Edge Cases:**
- Plugin consumes all fuel → runtime pauses plugin, resumes on next tick (not killed)
- Plugin exceeds memory → instance killed, error reported
- Plugin calls unreachable WASM instruction → trap caught

**Technical Notes:**
- Use `wasmtime` crate with `Engine::new()` and `Linker`
- Fuel: `store.set_fuel(100_000)?` then `store.consume_fuel()` in a loop
- Memory: `wasmtime::MemoryType::new(10_000, Some(10_000))` (10 pages ≈ 10MB)
- Each plugin instance gets its own `Store` and `Instance`

**Priority:** P1

#### US-6.3.2 — Host Function API (Imports)
**As a** developer (plugin author)
**I want** a set of host functions available to my WASM plugin
**so that** I can interact with the system and Dynamic Island.

**Acceptance Criteria:**
- Host API functions available as WASM imports:
  - `log_info(msg: string)`, `log_error(msg: string)`
  - `emit_activity(activity_json: string)` → push to activity queue
  - `send_command(command_json: string)` → send IPC to UI
  - `get_config(key: string) -> string` → read plugin config
  - `set_config(key: string, value: string)`
  - `on_event(event_type: string, callback: fn)` → subscribe to daemon events
  - `http_request(url: string, options: json) -> response` → HTTP client (if permitted)
  - `dbus_call(service: string, path: string, interface: string, method: string, args: json) -> json` → D-Bus (if permitted)
- Each function checks manifest permissions before executing
- Functions that require permissions throw error if not granted

**Edge Cases:**
- Plugin tries D-Bus call without permission → permission denied error
- Plugin passes malformed JSON → error returned, plugin not crashed
- HTTP request times out (>10s) → timeout error returned

**Technical Notes:**
- WASM imports are registered via `Linker::new()` with typed functions
- Complex types (strings, JSON) pass through linear memory: plugin allocates memory, writes data, passes pointer + length
- Helper crate `dyd-plugin-sdk` for Rust-based plugin development
- Example SDK usage:
  ```rust
  use dyd_plugin_sdk::*;

  #[no_mangle]
  pub fn init() -> i32 {
      log_info("docker plugin initialized");
      0 // success
  }

  #[no_mangle]
  pub fn on_event(event_ptr: *const u8, event_len: usize) -> i32 {
      let event = read_event(event_ptr, event_len);
      if event.event_type == "timer::tick" {
          let containers = check_docker();
          emit_activity(&serde_json::json!({
              "source": "docker",
              "content": containers
          }));
      }
      0
  }
  ```

**Priority:** P1

#### US-6.3.3 — Plugin UI Contribution
**As a** developer (plugin author)
**I want** my plugin to provide custom UI in the expanded Dynamic Island view
**so that** users can interact with my plugin's content.

**Acceptance Criteria:**
- Plugin can register UI component definitions in its manifest:
  - `component_type: "expanded"` → custom view for expanded island
  - `component_type: "icon"` → custom icon in status row
- UI components are defined as pre-built component types (not arbitrary HTML):
  - `table`: data table
  - `progress`: progress bar
  - `badge`: status badge
  - `list`: item list
  - `custom`: inline HTML (limited, sandboxed)
- Plugin sends structured data, daemon converts to UI component
- Security: no arbitrary JavaScript execution from plugins

**Edge Cases:**
- Plugin sends malformed data → show error placeholder
- Plugin component type not recognized → ignore

**Technical Notes:**
- UI component data is structured JSON sent via IPC
- UI renders known component types from plugin data
- For `custom` type: render HTML in a sandboxed iframe (or skip for v1)
- Plugin → daemon → UI data flow:
  ```
  Plugin (WASM) → Host function `emit_activity` → Daemon (IPC) → UI (React)
  ```
- UI component rendering:
  ```typescript
  interface PluginUIComponent {
      type: 'table' | 'progress' | 'badge' | 'list' | 'custom'
      data: Record<string, unknown>
  }
  ```

**Priority:** P2

---

## Feature 6.4: Plugin SDK & Tooling

**Description:** Provide a Rust SDK crate (`dyd-plugin-sdk`) and build tooling for plugin development. The SDK simplifies WASM plugin creation with macros, helpers, and type definitions.

**Acceptance Criteria:** SDK crate published. Example plugin compiles to WASM. `cargo build --target wasm32-wasi` produces a working plugin. SDK handles memory management for host function calls.

**Dependencies:** 6.3

**Notes:** Target triple: `wasm32-wasi` for standard WASI support. SDK should feel idiomatic for Rust developers.

### User Stories

#### US-6.4.1 — Plugin SDK Crate
**As a** developer (plugin author)
**I want** a Rust SDK that makes WASM plugin development ergonomic
**so that** I can focus on my plugin logic, not WASM glue code.

**Acceptance Criteria:**
- SDK provides:
  - `#[plugin]` proc macro to define plugin entry points
  - `log_info!`, `log_error!` macros
  - `emit_activity!` macro
  - `subscribe_event!` macro
  - HTTP client helper `http::get()`, `http::post()`
  - D-Bus helper `dbus::call()`
  - Config helpers `config::get()`, `config::set()`
  - Serialization helpers for host function arguments
- SDK is a `no_std` crate (WASM target may not have std)
- Generated WASM binary is <500KB for a minimal plugin

**Edge Cases:**
- Plugin uses `std` → wasm32-wasi target provides WASI preview 1 support
- SDK version mismatch with daemon → manifest declares `sdk_version`

**Technical Notes:**
- Crate structure: `dyd-plugin-sdk` with `dyd-plugin-macros` for proc macros
- WASM imports are resolved through SDK (low-level ABI hidden from plugin author)
- Plugin author's code:
  ```rust
  use dyd_plugin_sdk::prelude::*;

  #[plugin]
  impl MyPlugin {
      #[on_init]
      fn init(&self) -> Result<()> {
          log_info!("plugin initialized");
          Ok(())
      }

      #[on_event("timer::every_10s")]
      fn on_tick(&self) -> Result<()> {
          let status = get_docker_status()?;
          emit_activity!("docker", status);
          Ok(())
      }
  }
  ```

**Priority:** P2

#### US-6.4.2 — Plugin Build Tooling
**As a** developer (plugin author)
**I want** tooling to build, test, and package my plugin
**so that** I can develop plugins efficiently.

**Acceptance Criteria:**
- `cargo-dyd` CLI tool (or cargo subcommand):
  - `cargo dyd init` → scaffold new plugin project
  - `cargo dyd build` → build WASM binary
  - `cargo dyd package` → create `.dyd` plugin package (zip with manifest + wasm)
  - `cargo dyd install` → install to local daemon
- Plugin package format: `.dyd` zip containing manifest.json, plugin.wasm, and optional assets

**Edge Cases:**
- Plugin build fails → clear error output
- Target wasm32-wasi not installed → suggest `rustup target add wasm32-wasi`

**Technical Notes:**
- Plugin package: `{name}-{version}.dyd` (zip)
- Contents:
  ```
  manifest.json
  plugin.wasm
  assets/        (optional: icons, static files)
  ```
- Install: unzip to `$XDG_DATA_HOME/dyd/plugins/{name}/`

**Priority:** P3

---

## Feature 6.5: Plugin Management UI

**Description:** Provide a settings panel in the Dynamic Island (or a separate settings window) for managing plugins: list installed plugins, enable/disable, view status, configure settings, and uninstall.

**Acceptance Criteria:** Settings shows list of installed plugins. Each plugin shows name, version, status (running/error/disabled), and health. User can enable, disable, configure, and uninstall plugins. Plugin configuration UI is auto-generated from `config_schema`.

**Dependencies:** 6.2, 6.4

**Notes:** Plugin management is part of the Settings Epic (E7), but defined here for completeness.

### User Stories

#### US-6.5.1 — Plugin List in Settings
**As an** end user
**I want** to see a list of installed plugins in the settings
**so that** I know what plugins are active.

**Acceptance Criteria:**
- Plugin list shows: name, version, status badge (green=running, yellow=error, gray=disabled)
- Clicking a plugin opens detail view
- Status updates in real-time
- Empty state: "No plugins installed. Browse the plugin marketplace."
- Plugin count shown in summary

**Edge Cases:**
- Plugin crashed → status shows error with timestamp

**Technical Notes:**
- Component: `<PluginList>` in settings
- Data from IPC: `GetPlugins` command

**Priority:** P2

#### US-6.5.2 — Plugin Enable/Disable/Uninstall
**As an** end user
**I want** to enable, disable, or uninstall plugins
**so that** I have full control over which plugins run.

**Acceptance Criteria:**
- Toggle switch to enable/disable plugin
- Disabled plugin is put in Stopped state, not unloaded
- Uninstall button (with confirmation dialog)
- Uninstall removes plugin files and unloads from memory
- Plugin in Error state shows "Disable" and "Reset" buttons (reset re-enables)

**Edge Cases:**
- Uninstall while plugin is running → stop first, then uninstall
- Uninstall of core plugin (bundled) → warning "This is a system plugin"

**Technical Notes:**
- IPC commands: `SetPluginEnabled { name, enabled }`, `UninstallPlugin { name }`
- Confirmation dialog for uninstall: "Remove [Plugin Name]? This cannot be undone."

**Priority:** P2

#### US-6.5.3 — Plugin Configuration UI
**As an** end user
**I want** to configure plugin settings from a UI
**so that** I don't need to edit JSON files.

**Acceptance Criteria:**
- Settings UI auto-generated from plugin's `config_schema` (JSON Schema)
- Supported form fields: string, number, boolean, enum (dropdown), array (tag input), object (nested)
- Each field shows its label and description from schema
- Changes are saved when user clicks "Apply" or on change (auto-save)
- "Reset to defaults" button

**Edge Cases:**
- Schema has required fields → validate before save
- Schema changes between plugin versions → old values merged with new defaults
- Read-only fields (from `config_schema.readOnly`) → display but not editable

**Technical Notes:**
- Use `react-jsonschema-form` or custom form renderer
- Component: `<PluginConfigForm schema={configSchema} values={configValues} onChange={handleSave} />`
- Save IPC: `SetPluginConfig { name, config }`
- Auto-save with 1-second debounce

**Priority:** P2

---

## Feature 6.6: Example Plugins

**Description:** Develop 3 reference plugins to validate the plugin API, demonstrate capabilities, and serve as templates for third-party developers.

**Acceptance Criteria:** Docker status plugin works. VS Code Git status plugin works. Terminal command notification plugin works. Each plugin is documented with source code.

**Dependencies:** 6.3, 6.4

**Notes:** These plugins will also exercise Edge Cases in the plugin API implementation.

### User Stories

#### US-6.6.1 — Docker Status Plugin
**As an** end user
**I want** to see Docker container status in the Dynamic Island
**so that** I can monitor running containers at a glance.

**Acceptance Criteria:**
- Plugin connects to Docker socket (Unix: `/var/run/docker.sock`)
- Shows: container count (running/total) in pill
- Expanded view shows: container list with name, status, uptime
- Updates every 10 seconds
- Permissions: `unix_socket:/var/run/docker.sock`

**Edge Cases:**
- Docker not running → show "Docker not available"
- Permission denied → show error

**Technical Notes:**
- Example plugin demonstrating: HTTP client (via Unix socket), timer, activity emission
- ~200 lines of Rust code

**Priority:** P2

#### US-6.6.2 — VS Code Git Status Plugin
**As an** end user
**I want** to see my active Git repository status in the Dynamic Island
**so that** I know my branch, dirty state, and commit status without switching to the terminal.

**Acceptance Criteria:**
- Plugin monitors VS Code window title (via D-Bus `org.gnome.Shell.Introspect`)
- Extracts: repo name, branch, dirty state (✗), commits ahead/behind
- Shows: [branch name] + [dirty count] in pill
- Updates when VS Code window title changes
- Permissions: `dbus:org.gnome.Shell.Introspect`

**Edge Cases:**
- VS Code not running → no indicator shown
- No Git repo open → show "No repo"

**Technical Notes:**
- Demonstrates: D-Bus introspection, string parsing, event-driven activity emission
- ~150 lines of Rust code

**Priority:** P3

#### US-6.6.3 — Terminal Command Notification Plugin
**As an** end user
**I want** to see a notification in the Dynamic Island when a long-running terminal command completes
**so that** I can multitask without watching the terminal.

**Acceptance Criteria:**
- Plugin listens for shell notification signals (e.g., `notify-send` from shell integration)
- Or: watches shell history for command completion markers
- Shows: command name + exit code + duration
- Click → focus the terminal window
- Permissions: `dbus:org.freedesktop.Notifications`

**Edge Cases:**
- Command succeeded (exit 0) → green checkmark
- Command failed (exit != 0) → red X

**Technical Notes:**
- Demonstrates: notification interception, command execution time tracking
- ~120 lines of Rust code

**Priority:** P3
