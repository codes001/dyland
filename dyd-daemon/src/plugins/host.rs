use std::path::Path;

use dyd_types::PluginManifest;

/// Outcome of a single `tick()` invocation against a plugin instance. The tick
/// task in `manager.rs` uses this to decide whether to report a crash (which
/// feeds the 6-2 auto-disable path) without holding the registry lock across
/// the blocking hop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    /// Plugin has no `tick` export, is not running, or ran without incident.
    Idle,
    /// Plugin ran its `tick` export successfully.
    Ran,
    /// Plugin exhausted its per-tick fuel budget. Not a crash — the instance is
    /// left intact and re-invoked next tick with a fresh budget.
    Paused,
    /// Plugin trapped or returned a fatal status during `tick`; the string is a
    /// human-readable reason. The tick task routes this to `report_crash`.
    Crashed(String),
}

/// Plugin lifecycle executor. Story 6-3 provides a WASM runtime implementation;
/// story 6-2 ships a no-op stub so lifecycle management can be developed and
/// tested independently of the sandbox. The executor owns its own per-plugin
/// instances keyed by name.
pub trait PluginExecutor: Send + Sync {
    fn init(&self, name: &str, manifest: &PluginManifest, dir: &Path) -> Result<(), String>;
    fn start(&self, name: &str) -> Result<(), String>;
    fn stop(&self, name: &str) -> Result<(), String>;
    fn unload(&self, name: &str) -> Result<(), String>;
    fn memory_usage(&self, name: &str) -> Option<u64>;
    /// Drive one tick of the plugin's periodic work. Defaults to `Idle` so the
    /// stub executor and any future executor without periodic work need not
    /// implement it.
    fn tick(&self, _name: &str) -> TickOutcome {
        TickOutcome::Idle
    }
}

/// No-op executor used until the WASM runtime (story 6-3) lands.
pub struct StubExecutor;

impl PluginExecutor for StubExecutor {
    fn init(&self, name: &str, _manifest: &PluginManifest, _dir: &Path) -> Result<(), String> {
        tracing::debug!(plugin = %name, "stub executor init");
        Ok(())
    }
    fn start(&self, name: &str) -> Result<(), String> {
        tracing::debug!(plugin = %name, "stub executor start");
        Ok(())
    }
    fn stop(&self, name: &str) -> Result<(), String> {
        tracing::debug!(plugin = %name, "stub executor stop");
        Ok(())
    }
    fn unload(&self, name: &str) -> Result<(), String> {
        tracing::debug!(plugin = %name, "stub executor unload");
        Ok(())
    }
    fn memory_usage(&self, _name: &str) -> Option<u64> {
        None
    }
}

/// A host API that plugins use to interact with the daemon. Story 6-3 registers
/// these as WASM imports. The trait exists so the SDK ABI and the daemon host
/// stay in sync and testable.
pub trait HostApi: Send + Sync {
    fn log(&self, plugin: &str, level: &'static str, message: &str);
    fn emit_activity(&self, plugin: &str, activity_json: &str) -> Result<(), String>;
    fn send_command(&self, plugin: &str, command_json: &str) -> Result<(), String>;
    fn get_config(&self, plugin: &str, key: &str) -> Result<String, String>;
    fn set_config(&self, plugin: &str, key: &str, value: &str) -> Result<(), String>;
    fn http_request(&self, plugin: &str, request_json: &str) -> Result<String, String>;
    fn dbus_call(&self, plugin: &str, request_json: &str) -> Result<String, String>;
}
