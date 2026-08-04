// Included into `mod tests` at the bottom of runtime.rs. Exercises the WASM
// runtime end-to-end (through the `PluginExecutor` trait) and the individual
// `"dyd"` host imports (via directly instantiated probe modules).

use super::*;
use crate::plugins::host::{PluginExecutor, TickOutcome};
use dyd_types::{IpcEvent, IpcPayload, PluginManifest};
use wasmtime::{Engine, Linker, Module, Store};

// --- shared helpers --------------------------------------------------------

fn test_ctx() -> (Arc<HostContext>, broadcast::Receiver<IpcEnvelope>) {
    let (tx, rx) = broadcast::channel(32);
    let ctx = Arc::new(HostContext::new(tx, tokio::runtime::Handle::current()));
    (ctx, rx)
}

fn manifest(name: &str, entry: &str, perms: &[&str]) -> PluginManifest {
    PluginManifest {
        schema_version: 1,
        name: name.to_string(),
        version: "0.1.0".to_string(),
        author: "test".to_string(),
        description: None,
        homepage: None,
        license: None,
        entry_point: entry.to_string(),
        permissions: perms.iter().map(|s| s.to_string()).collect(),
        ui_components: Vec::new(),
        dependencies: Vec::new(),
        config_schema: None,
    }
}

/// Compile `wat` and write it as `plugin.wasm` into a fresh temp dir.
fn write_plugin(wat: &str) -> std::path::PathBuf {
    let bytes = wat::parse_str(wat).expect("wat should compile");
    let dir = std::env::temp_dir().join(format!("dyd-rt-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    dir
}

/// Directly instantiate a probe module against the real host functions, so
/// individual ABI calls can be asserted with precise return codes.
fn instantiate_probe(
    wat: &str,
    perms: &[&str],
    ctx: Arc<HostContext>,
) -> (Store<StoreState>, Instance) {
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let mut linker: Linker<StoreState> = Linker::new(&engine);
    register_host_functions(&mut linker).unwrap();
    let state = StoreState {
        limits: StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT_BYTES)
            .build(),
        ctx,
        plugin: "probe".to_string(),
        perms: PermSet::from_manifest(&perms.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
    };
    let mut store = Store::new(&engine, state);
    store.limiter(|s| &mut s.limits);
    store.set_fuel(LIFECYCLE_FUEL).unwrap();
    let bytes = wat::parse_str(wat).unwrap();
    let module = Module::new(&engine, &bytes).unwrap();
    let instance = linker.instantiate(&mut store, &module).unwrap();
    (store, instance)
}

fn call_probe(store: &mut Store<StoreState>, instance: &Instance, name: &str) -> i32 {
    let f = instance
        .get_typed_func::<(), i32>(&mut *store, name)
        .unwrap();
    f.call(&mut *store, ()).unwrap()
}

// A minimal well-behaved plugin: exports memory + a `start` returning 0. No
// `tick`/`init` — those must be tolerated as absent.
const MINIMAL: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "start") (result i32) (i32.const 0)))
"#;

// --- executor-level tests --------------------------------------------------

#[tokio::test]
async fn start_ok_and_missing_tick_tolerated() {
    let (ctx, _rx) = test_ctx();
    let exec = WasmExecutor::new(ctx).unwrap();
    let dir = write_plugin(MINIMAL);
    exec.init("m", &manifest("m", "plugin.wasm", &[]), &dir).unwrap();
    exec.start("m").unwrap();
    // No `tick` export -> Idle, never a crash.
    assert_eq!(exec.tick("m"), TickOutcome::Idle);
    exec.stop("m").unwrap();
    exec.unload("m").unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn two_plugins_are_isolated() {
    let (ctx, _rx) = test_ctx();
    let exec = WasmExecutor::new(ctx).unwrap();
    let d1 = write_plugin(MINIMAL);
    let d2 = write_plugin(MINIMAL);
    exec.init("a", &manifest("a", "plugin.wasm", &[]), &d1).unwrap();
    exec.init("b", &manifest("b", "plugin.wasm", &[]), &d2).unwrap();
    // Each has its own instance/memory; both start independently.
    exec.start("a").unwrap();
    exec.start("b").unwrap();
    assert!(exec.memory_usage("a").is_some());
    assert!(exec.memory_usage("b").is_some());
    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
}

#[tokio::test]
async fn memory_cap_rejects_oversized_initial_memory() {
    let (ctx, _rx) = test_ctx();
    let exec = WasmExecutor::new(ctx).unwrap();
    // 200 pages = 12.8 MiB initial > 10 MiB cap -> instantiation must fail.
    // (Regression against the story's 640 MiB `MemoryType::new(10_000, ..)`.)
    let big = r#"(module (memory (export "memory") 200) (func (export "start") (result i32) (i32.const 0)))"#;
    let dir = write_plugin(big);
    let err = exec.init("big", &manifest("big", "plugin.wasm", &[]), &dir).unwrap_err();
    assert!(err.contains("instantiation failed"), "unexpected: {err}");
    // 100 pages = 6.4 MiB is under the cap and succeeds.
    let ok = r#"(module (memory (export "memory") 100) (func (export "start") (result i32) (i32.const 0)))"#;
    let dir2 = write_plugin(ok);
    exec.init("ok", &manifest("ok", "plugin.wasm", &[]), &dir2).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[tokio::test]
async fn tick_out_of_fuel_pauses_not_crashes() {
    let (ctx, _rx) = test_ctx();
    let exec = WasmExecutor::new(ctx).unwrap();
    // `tick` spins forever -> exhausts the 100k tick-fuel budget -> Paused.
    let spin = r#"
      (module
        (memory (export "memory") 1)
        (func (export "start") (result i32) (i32.const 0))
        (func (export "tick") (result i32) (loop (br 0)) (i32.const 0)))
    "#;
    let dir = write_plugin(spin);
    exec.init("spin", &manifest("spin", "plugin.wasm", &[]), &dir).unwrap();
    exec.start("spin").unwrap();
    assert_eq!(exec.tick("spin"), TickOutcome::Paused);
    // Instance survives and can be re-ticked (still Paused, not gone).
    assert_eq!(exec.tick("spin"), TickOutcome::Paused);
    assert!(exec.memory_usage("spin").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn tick_trap_reports_crash() {
    let (ctx, _rx) = test_ctx();
    let exec = WasmExecutor::new(ctx).unwrap();
    let boom = r#"
      (module
        (memory (export "memory") 1)
        (func (export "start") (result i32) (i32.const 0))
        (func (export "tick") (result i32) (unreachable)))
    "#;
    let dir = write_plugin(boom);
    exec.init("boom", &manifest("boom", "plugin.wasm", &[]), &dir).unwrap();
    exec.start("boom").unwrap();
    assert!(matches!(exec.tick("boom"), TickOutcome::Crashed(_)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn emit_activity_broadcasts() {
    let (ctx, mut rx) = test_ctx();
    let exec = WasmExecutor::new(ctx).unwrap();
    // `start` emits a small JSON activity via the host import.
    let emit = r#"
      (module
        (import "dyd" "emit_activity" (func $emit (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (data (i32.const 16) "{\"k\":1}")
        (func (export "start") (result i32)
          (call $emit (i32.const 16) (i32.const 7))))
    "#;
    let dir = write_plugin(emit);
    exec.init("e", &manifest("e", "plugin.wasm", &[]), &dir).unwrap();
    exec.start("e").unwrap();
    let env = rx.try_recv().expect("an event should have been broadcast");
    match env.payload {
        IpcPayload::Event(IpcEvent::PluginActivity(a)) => {
            assert_eq!(a.source, "e");
            assert_eq!(a.content, serde_json::json!({"k": 1}));
        }
        other => panic!("unexpected payload: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// --- host-function-level tests --------------------------------------------

const SEND_CMD_PROBE: &str = r#"
(module
  (import "dyd" "send_command" (func $sc (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "{\"type\":\"Ping\"}")
  (func (export "probe") (result i32) (call $sc (i32.const 0) (i32.const 15))))
"#;

#[tokio::test]
async fn send_command_permission_gate() {
    let (ctx, _rx) = test_ctx();
    // No `api:` grant -> denied.
    let (mut s, i) = instantiate_probe(SEND_CMD_PROBE, &[], ctx.clone());
    assert_eq!(call_probe(&mut s, &i, "probe"), STATUS_DENIED);
    // With `api:` grant -> accepted (0).
    let (mut s2, i2) = instantiate_probe(SEND_CMD_PROBE, &["api:media"], ctx);
    assert_eq!(call_probe(&mut s2, &i2, "probe"), 0);
}

#[tokio::test]
async fn config_roundtrip_and_out_buffer_grow() {
    let (ctx, _rx) = test_ctx();
    // Seed an object so set_config can insert into it.
    ctx.set_config("probe", serde_json::json!({}));
    let wat = r#"
    (module
      (import "dyd" "set_config" (func $set (param i32 i32 i32 i32) (result i32)))
      (import "dyd" "get_config" (func $get (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "foo")
      (data (i32.const 8) "42")
      (func (export "do_set") (result i32)
        (call $set (i32.const 0) (i32.const 3) (i32.const 8) (i32.const 2)))
      (func (export "do_get") (result i32)
        (call $get (i32.const 0) (i32.const 3) (i32.const 64) (i32.const 16)))
      (func (export "do_get_small") (result i32)
        (call $get (i32.const 0) (i32.const 3) (i32.const 64) (i32.const 1)))
      (func (export "byte64") (result i32) (i32.load8_u (i32.const 64)))
      (func (export "byte65") (result i32) (i32.load8_u (i32.const 65))))
    "#;
    let (mut s, i) = instantiate_probe(wat, &[], ctx.clone());
    assert_eq!(call_probe(&mut s, &i, "do_set"), 0);
    // get returns needed length = 2, and writes "42" at offset 64.
    assert_eq!(call_probe(&mut s, &i, "do_get"), 2);
    assert_eq!(call_probe(&mut s, &i, "byte64"), b'4' as i32);
    assert_eq!(call_probe(&mut s, &i, "byte65"), b'2' as i32);
    // Too-small out buffer: returns needed size (2), writes nothing.
    assert_eq!(call_probe(&mut s, &i, "do_get_small"), 2);
    // Host-side config actually updated.
    let cfg = ctx.configs.read().unwrap().get("probe").cloned().unwrap();
    assert_eq!(cfg, serde_json::json!({"foo": 42}));
}

#[tokio::test]
async fn get_config_missing_key_returns_not_found() {
    let (ctx, _rx) = test_ctx();
    ctx.set_config("probe", serde_json::json!({}));
    let wat = r#"
    (module
      (import "dyd" "get_config" (func $get (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "nope")
      (func (export "probe") (result i32)
        (call $get (i32.const 0) (i32.const 4) (i32.const 64) (i32.const 16))))
    "#;
    let (mut s, i) = instantiate_probe(wat, &[], ctx);
    assert_eq!(call_probe(&mut s, &i, "probe"), STATUS_NOT_FOUND);
}

#[tokio::test]
async fn malformed_json_returns_minus_two() {
    let (ctx, _rx) = test_ctx();
    // emit_activity with non-JSON payload -> -2, never a trap (AC8).
    let wat = r#"
    (module
      (import "dyd" "emit_activity" (func $emit (param i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "not json {{{")
      (func (export "probe") (result i32) (call $emit (i32.const 0) (i32.const 12))))
    "#;
    let (mut s, i) = instantiate_probe(wat, &[], ctx);
    assert_eq!(call_probe(&mut s, &i, "probe"), STATUS_MALFORMED);
}

#[tokio::test]
async fn out_of_bounds_pointer_returns_minus_three() {
    let (ctx, _rx) = test_ctx();
    // log_info with a length running past the single 64 KiB page -> -3.
    let wat = r#"
    (module
      (import "dyd" "log_info" (func $log (param i32 i32) (result i32)))
      (memory (export "memory") 1)
      (func (export "probe") (result i32) (call $log (i32.const 0) (i32.const 200000))))
    "#;
    let (mut s, i) = instantiate_probe(wat, &[], ctx);
    assert_eq!(call_probe(&mut s, &i, "probe"), STATUS_BAD_MEMORY);
}

#[tokio::test]
async fn dbus_call_permission_gate_and_stub() {
    let (ctx, _rx) = test_ctx();
    let wat = r#"
    (module
      (import "dyd" "dbus_call" (func $d (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (data (i32.const 0) "{\"service\":\"org.x\"}")
      (func (export "probe") (result i32)
        (call $d (i32.const 0) (i32.const 19) (i32.const 128) (i32.const 16))))
    "#;
    // Without dbus grant -> denied.
    let (mut s, i) = instantiate_probe(wat, &[], ctx.clone());
    assert_eq!(call_probe(&mut s, &i, "probe"), STATUS_DENIED);
    // With grant -> unimplemented stub (-5).
    let (mut s2, i2) = instantiate_probe(wat, &["dbus:org.x"], ctx);
    assert_eq!(call_probe(&mut s2, &i2, "probe"), STATUS_UNIMPLEMENTED);
}

// --- permission unit tests -------------------------------------------------

#[test]
fn permset_http_matching() {
    let p = PermSet::from_manifest(&["https:api.github.com".to_string(), "http:*".to_string()]);
    assert!(p.allows_http("https://api.github.com/repos"));
    assert!(p.allows_http("http://anything.local/x"));
    assert!(!p.allows_http("https://evil.example.com"));
    assert!(!p.allows_http("ftp://api.github.com"));
}

#[test]
fn permset_dbus_and_api() {
    let p = PermSet::from_manifest(&["dbus:org.freedesktop.Notifications".to_string(), "api:media".to_string()]);
    assert!(p.allows_dbus("org.freedesktop.Notifications"));
    assert!(!p.allows_dbus("org.other"));
    assert!(p.allows_api());
    let q = PermSet::from_manifest(&["dbus:*".to_string()]);
    assert!(q.allows_dbus("anything"));
    assert!(!q.allows_api());
}
