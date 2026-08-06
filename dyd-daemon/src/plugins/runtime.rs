//! WASM runtime for the plugin system (story 6-3).
//!
//! Replaces the [`StubExecutor`](super::host::StubExecutor) with a real
//! `wasmtime` sandbox. Each plugin is compiled once and instantiated into its
//! own [`Store`], giving hard isolation between plugins (no shared linear
//! memory, no shared globals). Every guest call is bounded by **fuel** (CPU)
//! and a **`StoreLimits`** memory cap so a misbehaving plugin can neither hang
//! nor exhaust host memory.
//!
//! ## ABI (import module `"dyd"`)
//!
//! Strings cross the boundary as `(ptr, len)` pairs into the guest's exported
//! linear `memory`. Host functions that *return* a string use the WASI
//! `path_*` idiom: the guest passes an out-buffer `(ptr, cap)` and the host
//! returns the number of bytes it *would* write. If that exceeds `cap` the
//! guest re-`alloc`s a larger buffer and calls again — the host stays
//! stateless and never re-enters the guest.
//!
//! Status codes (returned by every host import):
//!
//! | code | meaning |
//! |------|---------|
//! | `>= 0` | ok (for string returns: bytes needed) |
//! | `-1` | permission denied |
//! | `-2` | malformed argument (e.g. invalid JSON) |
//! | `-3` | bad memory range (OOB `ptr`/`len`) |
//! | `-4` | host error (I/O, upstream failure) |
//! | `-5` | unimplemented |
//! | `-6` | not found |
//!
//! Malformed JSON and out-of-bounds pointers return a status; they never trap
//! (AC8). Only fuel exhaustion and genuine guest faults produce traps, which
//! the executor classifies into `Paused` (out-of-fuel on `tick`) vs `Crashed`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use dyd_types::{ActivityPayload, IpcCommand, IpcEnvelope, IpcEvent};
use tokio::sync::broadcast;
use wasmtime::{
    Caller, Engine, Extern, Instance, Linker, Memory, Module, Store, StoreLimits,
    StoreLimitsBuilder, Trap,
};

use super::host::{PluginExecutor, TickOutcome};

// ---------------------------------------------------------------------------
// Budgets & limits
// ---------------------------------------------------------------------------

/// Fuel granted to a single `tick()` call. Exhaustion pauses (not kills) the
/// plugin: it is re-invoked next tick with a fresh budget.
const TICK_FUEL: u64 = 100_000;
/// Fuel granted to lifecycle calls (`init`/`start`/`stop`/`unload`), which may
/// legitimately do more setup work than a tick.
const LIFECYCLE_FUEL: u64 = 5_000_000;
/// Hard cap on a plugin's linear memory (10 MiB of *actual* bytes). NB: this is
/// enforced via [`StoreLimits`], **not** a `MemoryType` maximum — a
/// `MemoryType::new(10_000, ..)` maximum would be 10_000 * 64 KiB ≈ 640 MiB.
const MEMORY_LIMIT_BYTES: usize = 10 * 1024 * 1024;
/// Upper bound on any single string a host function will accept or return, to
/// keep a hostile `len` from allocating unbounded host memory.
const MAX_STRING_BYTES: usize = 1024 * 1024;

// Status codes shared byte-for-byte with the SDK (story 6-4).
const STATUS_DENIED: i32 = -1;
const STATUS_MALFORMED: i32 = -2;
const STATUS_BAD_MEMORY: i32 = -3;
const STATUS_HOST_ERROR: i32 = -4;
const STATUS_UNIMPLEMENTED: i32 = -5;
const STATUS_NOT_FOUND: i32 = -6;

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

/// Parsed permission grants from a plugin manifest. Permission strings are
/// `"<scheme>:<target>"` (e.g. `"https:api.github.com"`, `"dbus:*"`,
/// `"api:media"`). A `*` target matches anything within that scheme.
#[derive(Debug, Clone, Default)]
pub struct PermSet {
    grants: Vec<(String, String)>,
}

impl PermSet {
    pub fn from_manifest(perms: &[String]) -> Self {
        let grants = perms
            .iter()
            .filter_map(|p| {
                p.split_once(':')
                    .map(|(s, t)| (s.to_string(), t.to_string()))
            })
            .collect();
        Self { grants }
    }

    fn scheme_target_matches(&self, scheme: &str, target: &str) -> bool {
        self.grants
            .iter()
            .any(|(s, t)| s == scheme && (t == "*" || t == target || target.ends_with(t.as_str())))
    }

    /// Any `api:` grant lets a plugin issue daemon commands.
    fn allows_api(&self) -> bool {
        self.grants.iter().any(|(s, _)| s == "api")
    }

    /// A URL is permitted if its scheme+host is granted (or `<scheme>:*`).
    fn allows_http(&self, url: &str) -> bool {
        let Some((scheme, rest)) = url.split_once("://") else {
            return false;
        };
        if scheme != "http" && scheme != "https" {
            return false;
        }
        let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
        let host = host.split('@').next_back().unwrap_or(host);
        let host = host.split(':').next().unwrap_or(host);
        self.scheme_target_matches(scheme, host)
    }

    fn allows_dbus(&self, service: &str) -> bool {
        self.scheme_target_matches("dbus", service)
    }
}

// ---------------------------------------------------------------------------
// Shared host context (owned by both the executor and the manager)
// ---------------------------------------------------------------------------

/// Data and channels the host functions need, shared behind an `Arc` by the
/// [`WasmExecutor`] and the [`PluginManager`](super::manager::PluginManager).
///
/// It deliberately holds **only** channels and plain data — never an
/// `Arc<PluginManager>` — so there is no reference cycle. The manager writes
/// plugin config through `configs`; guest `get_config`/`set_config` read and
/// write the same map.
pub struct HostContext {
    /// Broadcast bus shared with the IPC server; plugin activity and commands
    /// are published here.
    pub event_broadcast: broadcast::Sender<IpcEnvelope>,
    /// Per-plugin configuration, keyed by plugin name. Values mirror each
    /// plugin's `config` in the registry.
    pub configs: RwLock<HashMap<String, serde_json::Value>>,
    /// Tokio handle used to `block_on` async host work (e.g. HTTP) from the
    /// blocking thread a guest call runs on.
    pub rt: tokio::runtime::Handle,
    /// Shared HTTP client with a baked-in request timeout.
    pub http: reqwest::Client,
}

impl HostContext {
    pub fn new(
        event_broadcast: broadcast::Sender<IpcEnvelope>,
        rt: tokio::runtime::Handle,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            event_broadcast,
            configs: RwLock::new(HashMap::new()),
            rt,
            http,
        }
    }

    /// Seed or replace a plugin's config (called by the manager on register and
    /// on `SetPluginConfig`).
    pub fn set_config(&self, plugin: &str, config: serde_json::Value) {
        if let Ok(mut map) = self.configs.write() {
            map.insert(plugin.to_string(), config);
        }
    }
}

// ---------------------------------------------------------------------------
// Per-store state
// ---------------------------------------------------------------------------

/// State attached to each plugin's [`Store`]. Host functions reach it via
/// `caller.data()`.
pub struct StoreState {
    limits: StoreLimits,
    ctx: Arc<HostContext>,
    plugin: String,
    perms: PermSet,
}

/// A single compiled + instantiated plugin. Guarded by a `std::sync::Mutex`
/// that is only ever locked on the `spawn_blocking` thread (never across an
/// `.await`), so a plain `Mutex` is correct and cheap.
struct SandboxedPlugin {
    store: Store<StoreState>,
    instance: Instance,
    memory: Memory,
}

/// Result of invoking a guest export.
enum CallResult {
    /// Export ran and returned this status code.
    Ok(i32),
    /// The export does not exist — tolerated for optional hooks.
    Missing,
    /// The call exhausted its fuel budget.
    OutOfFuel,
    /// The call trapped for some other reason.
    Trap(String),
}

impl SandboxedPlugin {
    /// Invoke a `() -> i32` guest export with a fresh fuel budget.
    fn call_no_arg(&mut self, name: &str, fuel: u64) -> CallResult {
        if let Err(e) = self.store.set_fuel(fuel) {
            return CallResult::Trap(format!("failed to set fuel: {e}"));
        }
        let func = match self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, name)
        {
            Ok(f) => f,
            Err(_) => return CallResult::Missing,
        };
        match func.call(&mut self.store, ()) {
            Ok(code) => CallResult::Ok(code),
            Err(err) => {
                if is_out_of_fuel(&err) {
                    CallResult::OutOfFuel
                } else {
                    CallResult::Trap(err.to_string())
                }
            }
        }
    }

    fn memory_bytes(&self) -> u64 {
        self.memory.data_size(&self.store) as u64
    }
}

fn is_out_of_fuel(err: &wasmtime::Error) -> bool {
    matches!(err.downcast_ref::<Trap>(), Some(Trap::OutOfFuel))
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

/// wasmtime-backed [`PluginExecutor`]. Owns one instance per plugin, keyed by
/// name.
pub struct WasmExecutor {
    engine: Engine,
    ctx: Arc<HostContext>,
    plugins: RwLock<HashMap<String, Arc<Mutex<SandboxedPlugin>>>>,
}

impl WasmExecutor {
    pub fn new(ctx: Arc<HostContext>) -> Result<Self, String> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).map_err(|e| format!("engine init failed: {e}"))?;
        Ok(Self {
            engine,
            ctx,
            plugins: RwLock::new(HashMap::new()),
        })
    }

    fn get(&self, name: &str) -> Option<Arc<Mutex<SandboxedPlugin>>> {
        self.plugins.read().ok()?.get(name).cloned()
    }

    /// Build a fresh linker with the `"dyd"` host imports registered. Built per
    /// `init` (a rare event) so that `define_unknown_imports_as_traps` can be
    /// applied against a specific module without sharing a mutable linker.
    fn build_linker(&self) -> Result<Linker<StoreState>, String> {
        let mut linker: Linker<StoreState> = Linker::new(&self.engine);
        register_host_functions(&mut linker)
            .map_err(|e| format!("failed to register host functions: {e}"))?;
        Ok(linker)
    }
}

impl PluginExecutor for WasmExecutor {
    fn init(
        &self,
        name: &str,
        manifest: &dyd_types::PluginManifest,
        dir: &Path,
    ) -> Result<(), String> {
        let wasm_path = dir.join(&manifest.entry_point);
        let bytes = std::fs::read(&wasm_path)
            .map_err(|e| format!("cannot read {}: {e}", wasm_path.display()))?;
        let module =
            Module::new(&self.engine, &bytes).map_err(|e| format!("invalid wasm module: {e}"))?;

        let mut linker = self.build_linker()?;
        // Tolerate stray imports (e.g. WASI stubs a toolchain may emit): they
        // become traps only if actually called, rather than failing linking.
        linker
            .define_unknown_imports_as_traps(&module)
            .map_err(|e| format!("failed to stub unknown imports: {e}"))?;

        let state = StoreState {
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY_LIMIT_BYTES)
                .build(),
            ctx: self.ctx.clone(),
            plugin: name.to_string(),
            perms: PermSet::from_manifest(&manifest.permissions),
        };
        let mut store = Store::new(&self.engine, state);
        store.limiter(|s| &mut s.limits);
        store
            .set_fuel(LIFECYCLE_FUEL)
            .map_err(|e| format!("failed to set fuel: {e}"))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("instantiation failed: {e}"))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| "plugin does not export `memory`".to_string())?;

        let mut plugin = SandboxedPlugin {
            store,
            instance,
            memory,
        };

        // Optional `init` hook.
        match plugin.call_no_arg("init", LIFECYCLE_FUEL) {
            CallResult::Ok(code) if code >= 0 => {}
            CallResult::Ok(code) => return Err(format!("init returned status {code}")),
            CallResult::Missing => {}
            CallResult::OutOfFuel => return Err("init exceeded fuel budget".to_string()),
            CallResult::Trap(t) => return Err(format!("init trapped: {t}")),
        }

        self.plugins
            .write()
            .map_err(|_| "plugin registry poisoned".to_string())?
            .insert(name.to_string(), Arc::new(Mutex::new(plugin)));
        Ok(())
    }

    fn start(&self, name: &str) -> Result<(), String> {
        lifecycle_call(self.get(name), name, "start")
    }

    fn stop(&self, name: &str) -> Result<(), String> {
        lifecycle_call(self.get(name), name, "stop")
    }

    fn unload(&self, name: &str) -> Result<(), String> {
        let result = lifecycle_call(self.get(name), name, "unload");
        if let Ok(mut map) = self.plugins.write() {
            map.remove(name);
        }
        result
    }

    fn memory_usage(&self, name: &str) -> Option<u64> {
        let plugin = self.get(name)?;
        // Never block the async caller: if a tick holds the lock, skip.
        let guard = plugin.try_lock().ok()?;
        Some(guard.memory_bytes())
    }

    fn tick(&self, name: &str) -> TickOutcome {
        let Some(plugin) = self.get(name) else {
            return TickOutcome::Idle;
        };
        let mut guard = match plugin.try_lock() {
            Ok(g) => g,
            // A previous tick is still running (or memory_usage briefly holds
            // it); treat as idle this round rather than blocking.
            Err(_) => return TickOutcome::Idle,
        };
        match guard.call_no_arg("tick", TICK_FUEL) {
            CallResult::Ok(code) if code >= 0 => TickOutcome::Ran,
            CallResult::Ok(code) => TickOutcome::Crashed(format!("tick returned status {code}")),
            CallResult::Missing => TickOutcome::Idle,
            CallResult::OutOfFuel => TickOutcome::Paused,
            CallResult::Trap(t) => TickOutcome::Crashed(format!("tick trapped: {t}")),
        }
    }
}

/// Shared lifecycle-call handling for start/stop/unload.
fn lifecycle_call(
    plugin: Option<Arc<Mutex<SandboxedPlugin>>>,
    name: &str,
    export: &str,
) -> Result<(), String> {
    let Some(plugin) = plugin else {
        // No live instance (e.g. init failed): treat lifecycle op as a no-op so
        // stop/unload stay idempotent.
        tracing::debug!(plugin = %name, export, "no instance; lifecycle no-op");
        return Ok(());
    };
    let mut guard = plugin
        .lock()
        .map_err(|_| format!("{name} instance lock poisoned"))?;
    match guard.call_no_arg(export, LIFECYCLE_FUEL) {
        CallResult::Ok(code) if code >= 0 => Ok(()),
        CallResult::Ok(code) => Err(format!("{export} returned status {code}")),
        CallResult::Missing => Ok(()),
        CallResult::OutOfFuel => Err(format!("{export} exceeded fuel budget")),
        CallResult::Trap(t) => Err(format!("{export} trapped: {t}")),
    }
}

// ---------------------------------------------------------------------------
// ABI memory helpers (bounds-checked)
// ---------------------------------------------------------------------------

fn memory_of(caller: &mut Caller<'_, StoreState>) -> Option<Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(m)) => Some(m),
        _ => None,
    }
}

/// Read a `(ptr, len)` byte range out of guest memory. Returns a status code on
/// out-of-bounds or oversized ranges — never traps.
fn read_bytes(caller: &mut Caller<'_, StoreState>, ptr: i32, len: i32) -> Result<Vec<u8>, i32> {
    if ptr < 0 || len < 0 {
        return Err(STATUS_BAD_MEMORY);
    }
    let (ptr, len) = (ptr as usize, len as usize);
    if len > MAX_STRING_BYTES {
        return Err(STATUS_MALFORMED);
    }
    let mem = memory_of(caller).ok_or(STATUS_BAD_MEMORY)?;
    let data = mem.data(&*caller);
    let end = ptr.checked_add(len).ok_or(STATUS_BAD_MEMORY)?;
    if end > data.len() {
        return Err(STATUS_BAD_MEMORY);
    }
    Ok(data[ptr..end].to_vec())
}

fn read_string(caller: &mut Caller<'_, StoreState>, ptr: i32, len: i32) -> Result<String, i32> {
    let bytes = read_bytes(caller, ptr, len)?;
    String::from_utf8(bytes).map_err(|_| STATUS_MALFORMED)
}

/// Write `data` into the guest out-buffer at `(ptr, cap)` if it fits, and
/// return the number of bytes needed. If `needed > cap` nothing is written and
/// the guest is expected to grow its buffer and retry. Returns a negative
/// status only on a bad pointer.
fn write_out(caller: &mut Caller<'_, StoreState>, ptr: i32, cap: i32, data: &[u8]) -> i32 {
    let needed = data.len();
    if needed > MAX_STRING_BYTES {
        return STATUS_HOST_ERROR;
    }
    if ptr < 0 || cap < 0 {
        return STATUS_BAD_MEMORY;
    }
    let (ptr, cap) = (ptr as usize, cap as usize);
    if needed > cap {
        // Signal required size; guest re-allocs and calls again.
        return needed as i32;
    }
    let Some(mem) = memory_of(caller) else {
        return STATUS_BAD_MEMORY;
    };
    let buf = mem.data_mut(&mut *caller);
    let Some(end) = ptr.checked_add(needed) else {
        return STATUS_BAD_MEMORY;
    };
    if end > buf.len() {
        return STATUS_BAD_MEMORY;
    }
    buf[ptr..end].copy_from_slice(data);
    needed as i32
}

// ---------------------------------------------------------------------------
// Host functions
// ---------------------------------------------------------------------------

/// Register all `"dyd"` imports on the linker. Kept as a free function so tests
/// can build an executor and exercise the same registrations.
fn register_host_functions(linker: &mut Linker<StoreState>) -> wasmtime::Result<()> {
    linker.func_wrap(
        "dyd",
        "log_info",
        |mut caller: Caller<'_, StoreState>, ptr: i32, len: i32| -> i32 {
            match read_string(&mut caller, ptr, len) {
                Ok(msg) => {
                    tracing::info!(plugin = %caller.data().plugin, "{msg}");
                    0
                }
                Err(code) => code,
            }
        },
    )?;

    linker.func_wrap(
        "dyd",
        "log_error",
        |mut caller: Caller<'_, StoreState>, ptr: i32, len: i32| -> i32 {
            match read_string(&mut caller, ptr, len) {
                Ok(msg) => {
                    tracing::warn!(plugin = %caller.data().plugin, "{msg}");
                    0
                }
                Err(code) => code,
            }
        },
    )?;

    linker.func_wrap(
        "dyd",
        "emit_activity",
        |mut caller: Caller<'_, StoreState>, ptr: i32, len: i32| -> i32 {
            let json = match read_string(&mut caller, ptr, len) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let content: serde_json::Value = match serde_json::from_str(&json) {
                Ok(v) => v,
                Err(_) => return STATUS_MALFORMED,
            };
            let state = caller.data();
            let envelope = IpcEnvelope::event(IpcEvent::PluginActivity(ActivityPayload {
                activity_id: uuid::Uuid::new_v4().to_string(),
                source: state.plugin.clone(),
                content,
            }));
            let _ = state.ctx.event_broadcast.send(envelope);
            0
        },
    )?;

    linker.func_wrap(
        "dyd",
        "send_command",
        |mut caller: Caller<'_, StoreState>, ptr: i32, len: i32| -> i32 {
            if !caller.data().perms.allows_api() {
                return STATUS_DENIED;
            }
            let json = match read_string(&mut caller, ptr, len) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let command: IpcCommand = match serde_json::from_str(&json) {
                Ok(c) => c,
                Err(_) => return STATUS_MALFORMED,
            };
            let state = caller.data();
            // Publish the command envelope on the shared bus; the daemon's command
            // routing consumes it (full round-trip wiring is deferred with dbus).
            let _ = state
                .ctx
                .event_broadcast
                .send(IpcEnvelope::command(command));
            0
        },
    )?;

    linker.func_wrap(
        "dyd",
        "get_config",
        |mut caller: Caller<'_, StoreState>, kptr: i32, klen: i32, optr: i32, ocap: i32| -> i32 {
            let key = match read_string(&mut caller, kptr, klen) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let state = caller.data();
            let value = {
                let map = match state.ctx.configs.read() {
                    Ok(m) => m,
                    Err(_) => return STATUS_HOST_ERROR,
                };
                map.get(&state.plugin)
                    .and_then(|cfg| cfg.get(&key))
                    .cloned()
            };
            let Some(value) = value else {
                return STATUS_NOT_FOUND;
            };
            let serialized = match serde_json::to_vec(&value) {
                Ok(b) => b,
                Err(_) => return STATUS_HOST_ERROR,
            };
            write_out(&mut caller, optr, ocap, &serialized)
        },
    )?;

    linker.func_wrap(
        "dyd",
        "set_config",
        |mut caller: Caller<'_, StoreState>, kptr: i32, klen: i32, vptr: i32, vlen: i32| -> i32 {
            let key = match read_string(&mut caller, kptr, klen) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let value_json = match read_string(&mut caller, vptr, vlen) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let value: serde_json::Value = match serde_json::from_str(&value_json) {
                Ok(v) => v,
                Err(_) => return STATUS_MALFORMED,
            };
            let state = caller.data();
            let mut map = match state.ctx.configs.write() {
                Ok(m) => m,
                Err(_) => return STATUS_HOST_ERROR,
            };
            let entry = map
                .entry(state.plugin.clone())
                .or_insert_with(|| serde_json::json!({}));
            if let Some(obj) = entry.as_object_mut() {
                obj.insert(key, value);
                0
            } else {
                STATUS_HOST_ERROR
            }
        },
    )?;

    // Event subscription registration — no-op stub for 6-3 (guest-directed
    // event delivery is not wired yet). Reads and ignores the payload.
    linker.func_wrap(
        "dyd",
        "on_event",
        |mut caller: Caller<'_, StoreState>, ptr: i32, len: i32| -> i32 {
            match read_bytes(&mut caller, ptr, len) {
                Ok(_) => 0,
                Err(code) => code,
            }
        },
    )?;

    linker.func_wrap(
        "dyd",
        "http_request",
        |mut caller: Caller<'_, StoreState>, rptr: i32, rlen: i32, optr: i32, ocap: i32| -> i32 {
            let json = match read_string(&mut caller, rptr, rlen) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let request: HttpRequest = match serde_json::from_str(&json) {
                Ok(r) => r,
                Err(_) => return STATUS_MALFORMED,
            };
            let state = caller.data();
            if !state.perms.allows_http(&request.url) {
                return STATUS_DENIED;
            }
            let ctx = state.ctx.clone();
            let response = ctx.rt.block_on(perform_http(&ctx.http, &request));
            match response {
                Ok(body) => write_out(&mut caller, optr, ocap, &body),
                Err(_) => STATUS_HOST_ERROR,
            }
        },
    )?;

    // D-Bus bridge — permission-gated stub returning `-5` for 6-3. Full wiring
    // (via zbus) is deferred; the SDK/examples degrade gracefully.
    linker.func_wrap(
        "dyd",
        "dbus_call",
        |mut caller: Caller<'_, StoreState>, rptr: i32, rlen: i32, _optr: i32, _ocap: i32| -> i32 {
            let json = match read_string(&mut caller, rptr, rlen) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let request: DbusRequest = match serde_json::from_str(&json) {
                Ok(r) => r,
                Err(_) => return STATUS_MALFORMED,
            };
            if !caller.data().perms.allows_dbus(&request.service) {
                return STATUS_DENIED;
            }
            STATUS_UNIMPLEMENTED
        },
    )?;

    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP / D-Bus request shapes
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct HttpRequest {
    url: String,
    #[serde(default = "default_method")]
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Debug, serde::Serialize)]
struct HttpResponse {
    status: u16,
    body: String,
}

async fn perform_http(client: &reqwest::Client, req: &HttpRequest) -> Result<Vec<u8>, String> {
    let method = reqwest::Method::from_bytes(req.method.to_uppercase().as_bytes())
        .map_err(|e| e.to_string())?;
    let mut builder = client.request(method, &req.url);
    for (k, v) in &req.headers {
        builder = builder.header(k, v);
    }
    if let Some(body) = &req.body {
        builder = builder.body(body.clone());
    }
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    let out = HttpResponse { status, body };
    serde_json::to_vec(&out).map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
struct DbusRequest {
    service: String,
    #[serde(default)]
    #[allow(dead_code)]
    path: String,
    #[serde(default)]
    #[allow(dead_code)]
    interface: String,
    #[serde(default)]
    #[allow(dead_code)]
    method: String,
}

#[cfg(test)]
mod tests {
    include!("runtime_tests.rs");
}
