//! # dyd-plugin-sdk
//!
//! Ergonomic `no_std` SDK for authoring [dyland](https://) Dynamic Island
//! plugins that compile to `wasm32-wasip1` and run inside the daemon's
//! `wasmtime` sandbox (see daemon story 6-3).
//!
//! The SDK hides the raw `(ptr, len)` WASM ABI behind Rust helpers and macros:
//!
//! ```ignore
//! use dyd_plugin_sdk::prelude::*;
//!
//! struct MyPlugin;
//! impl Plugin for MyPlugin {
//!     fn tick(&mut self) {
//!         log_info!("hello from {}", "my-plugin");
//!         emit_activity!(json!({ "text": "tick" }));
//!     }
//! }
//! plugin_entry!(MyPlugin);
//! ```
//!
//! Host functions return an `i32` status: `>= 0` ok (bytes needed for string
//! returns), `-1` denied, `-2` malformed, `-3` bad memory, `-4` host error,
//! `-5` unimplemented, `-6` not found. The byte-for-byte ABI is shared with the
//! daemon's `"dyd"` import module.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

// --- allocator + panic handler (real wasm plugin builds only) --------------
// On the host (unit tests) std provides both, so these are compiled out.
#[cfg(all(target_arch = "wasm32", not(test)))]
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// Minimal `mem*` intrinsics. A `no_std` wasm cdylib does not pull in
// `compiler_builtins`' `mem` feature, yet `serde_json`/`core` reference
// `memcmp`/`memcpy`/`memset`/`memmove`. Providing them here lets plugin authors
// build a normal `cargo build --target wasm32-wasip1` without `-Zbuild-std`.
// wasm32 is single-threaded, so these plain byte loops are correct and small.
#[cfg(all(target_arch = "wasm32", not(test)))]
mod mem_intrinsics {
    #[no_mangle]
    pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        let mut i = 0;
        while i < n {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
        dest
    }

    #[no_mangle]
    pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        if (dest as usize) < (src as usize) {
            let mut i = 0;
            while i < n {
                *dest.add(i) = *src.add(i);
                i += 1;
            }
        } else {
            let mut i = n;
            while i > 0 {
                i -= 1;
                *dest.add(i) = *src.add(i);
            }
        }
        dest
    }

    #[no_mangle]
    pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
        let mut i = 0;
        while i < n {
            *dest.add(i) = c as u8;
            i += 1;
        }
        dest
    }

    #[no_mangle]
    pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
        let mut i = 0;
        while i < n {
            let x = *a.add(i);
            let y = *b.add(i);
            if x != y {
                return x as i32 - y as i32;
            }
            i += 1;
        }
        0
    }
}

// --- status codes (shared with the daemon) ---------------------------------

/// Permission denied for the requested operation.
pub const STATUS_DENIED: i32 = -1;
/// A malformed argument (e.g. invalid JSON).
pub const STATUS_MALFORMED: i32 = -2;
/// A bad `(ptr, len)` memory range.
pub const STATUS_BAD_MEMORY: i32 = -3;
/// A host-side error (I/O, upstream failure).
pub const STATUS_HOST_ERROR: i32 = -4;
/// The host function is not implemented (e.g. the D-Bus stub in 6-3).
pub const STATUS_UNIMPLEMENTED: i32 = -5;
/// The requested item was not found (e.g. a missing config key).
pub const STATUS_NOT_FOUND: i32 = -6;

// --- raw host imports ------------------------------------------------------

mod sys {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "dyd")]
    extern "C" {
        pub fn log_info(ptr: i32, len: i32) -> i32;
        pub fn log_error(ptr: i32, len: i32) -> i32;
        pub fn emit_activity(ptr: i32, len: i32) -> i32;
        pub fn send_command(ptr: i32, len: i32) -> i32;
        pub fn get_config(kptr: i32, klen: i32, optr: i32, ocap: i32) -> i32;
        pub fn set_config(kptr: i32, klen: i32, vptr: i32, vlen: i32) -> i32;
        pub fn on_event(ptr: i32, len: i32) -> i32;
        pub fn http_request(rptr: i32, rlen: i32, optr: i32, ocap: i32) -> i32;
        pub fn dbus_call(rptr: i32, rlen: i32, optr: i32, ocap: i32) -> i32;
    }

    // Host-side stand-ins so the crate links and unit-tests on the dev machine.
    #[cfg(not(target_arch = "wasm32"))]
    mod stub {
        pub unsafe extern "C" fn log_info(_ptr: i32, _len: i32) -> i32 {
            0
        }
        pub unsafe extern "C" fn log_error(_ptr: i32, _len: i32) -> i32 {
            0
        }
        pub unsafe extern "C" fn emit_activity(_ptr: i32, _len: i32) -> i32 {
            0
        }
        pub unsafe extern "C" fn send_command(_ptr: i32, _len: i32) -> i32 {
            0
        }
        pub unsafe extern "C" fn get_config(_kptr: i32, _klen: i32, _optr: i32, _ocap: i32) -> i32 {
            super::super::STATUS_NOT_FOUND
        }
        pub unsafe extern "C" fn set_config(_kptr: i32, _klen: i32, _vptr: i32, _vlen: i32) -> i32 {
            0
        }
        pub unsafe extern "C" fn on_event(_ptr: i32, _len: i32) -> i32 {
            0
        }
        pub unsafe extern "C" fn http_request(_rptr: i32, _rlen: i32, _optr: i32, _ocap: i32) -> i32 {
            super::super::STATUS_UNIMPLEMENTED
        }
        pub unsafe extern "C" fn dbus_call(_rptr: i32, _rlen: i32, _optr: i32, _ocap: i32) -> i32 {
            super::super::STATUS_UNIMPLEMENTED
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub use stub::*;
}

// --- memory management (exported by the plugin via `plugin_entry!`) ---------

/// Allocate `len` bytes in the plugin's linear memory and return a raw pointer.
/// Called by the host before writing an argument the guest will read. Exposed
/// through the `alloc` export emitted by [`plugin_entry!`].
#[doc(hidden)]
pub fn alloc_impl(len: i32) -> i32 {
    if len <= 0 {
        return 0;
    }
    let len = len as usize;
    let layout = core::alloc::Layout::from_size_align(len, 1).unwrap();
    unsafe {
        let ptr = alloc::alloc::alloc(layout);
        if ptr.is_null() {
            0
        } else {
            ptr as i32
        }
    }
}

/// Free a previously [`alloc_impl`]'d region. Exposed through the `dealloc`
/// export emitted by [`plugin_entry!`].
#[doc(hidden)]
pub fn dealloc_impl(ptr: i32, len: i32) {
    if ptr == 0 || len <= 0 {
        return;
    }
    let len = len as usize;
    let layout = core::alloc::Layout::from_size_align(len, 1).unwrap();
    unsafe {
        alloc::alloc::dealloc(ptr as *mut u8, layout);
    }
}

// --- ABI marshalling helpers -----------------------------------------------

/// Invoke a host import that takes a single `(ptr, len)` byte-slice argument.
fn call_bytes(f: unsafe extern "C" fn(i32, i32) -> i32, data: &[u8]) -> i32 {
    unsafe { f(data.as_ptr() as i32, data.len() as i32) }
}

/// Invoke a host import that fills a caller-provided out-buffer, growing and
/// retrying on the WASI `path_*` "needed length" convention. Returns the bytes
/// written on success, or `Err(status)` on a negative status.
fn call_out_buffer(mut f: impl FnMut(i32, i32) -> i32) -> Result<Vec<u8>, i32> {
    let mut cap = 256usize;
    loop {
        let mut buf = vec![0u8; cap];
        let ret = f(buf.as_mut_ptr() as i32, cap as i32);
        if ret < 0 {
            return Err(ret);
        }
        let needed = ret as usize;
        if needed <= cap {
            buf.truncate(needed);
            return Ok(buf);
        }
        // Host reported it needs more room; grow and retry once.
        cap = needed;
    }
}

// --- logging ---------------------------------------------------------------

/// Log an informational message to the daemon.
pub fn log_info(msg: &str) {
    call_bytes(sys::log_info, msg.as_bytes());
}

/// Log an error message to the daemon.
pub fn log_error(msg: &str) {
    call_bytes(sys::log_error, msg.as_bytes());
}

// --- activities ------------------------------------------------------------

/// Emit an activity (arbitrary JSON) to the Dynamic Island. Returns the host
/// status code (`0` on success).
pub fn emit_activity(value: &serde_json::Value) -> i32 {
    match serde_json::to_vec(value) {
        Ok(bytes) => call_bytes(sys::emit_activity, &bytes),
        Err(_) => STATUS_MALFORMED,
    }
}

/// Send a structured [`serde_json::Value`] command to the daemon. Requires an
/// `api:` permission grant; returns `-1` if not granted.
pub fn send_command(value: &serde_json::Value) -> i32 {
    match serde_json::to_vec(value) {
        Ok(bytes) => call_bytes(sys::send_command, &bytes),
        Err(_) => STATUS_MALFORMED,
    }
}

/// Register interest in a named daemon event. Event delivery is a host stub in
/// this SDK version; the call is accepted and reserved for future wiring.
pub fn subscribe_event(name: &str) -> i32 {
    call_bytes(sys::on_event, name.as_bytes())
}

// --- config ----------------------------------------------------------------

/// Configuration accessors backed by the daemon's per-plugin config map.
pub mod config {
    use super::*;

    /// Read a config value by key. Returns `None` if the key is absent.
    pub fn get(key: &str) -> Option<serde_json::Value> {
        let result = call_out_buffer(|optr, ocap| unsafe {
            sys::get_config(key.as_ptr() as i32, key.len() as i32, optr, ocap)
        });
        match result {
            Ok(bytes) => serde_json::from_slice(&bytes).ok(),
            Err(_) => None,
        }
    }

    /// Write a config value by key. Returns the host status (`0` on success).
    pub fn set(key: &str, value: &serde_json::Value) -> i32 {
        let bytes = match serde_json::to_vec(value) {
            Ok(b) => b,
            Err(_) => return STATUS_MALFORMED,
        };
        unsafe {
            sys::set_config(
                key.as_ptr() as i32,
                key.len() as i32,
                bytes.as_ptr() as i32,
                bytes.len() as i32,
            )
        }
    }
}

// --- HTTP ------------------------------------------------------------------

/// HTTP helpers. Requires an `http:`/`https:` permission grant for the target
/// host; otherwise calls return `Err(-1)`.
pub mod http {
    use super::*;
    use alloc::string::String;
    use alloc::string::ToString;

    /// A host HTTP response.
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct Response {
        /// HTTP status code.
        pub status: u16,
        /// Response body as a string.
        pub body: String,
    }

    fn request(method: &str, url: &str, body: Option<&str>) -> Result<Response, i32> {
        let req = serde_json::json!({
            "url": url,
            "method": method,
            "body": body,
        });
        let bytes = serde_json::to_vec(&req).map_err(|_| STATUS_MALFORMED)?;
        let out = call_out_buffer(|optr, ocap| unsafe {
            sys::http_request(bytes.as_ptr() as i32, bytes.len() as i32, optr, ocap)
        })?;
        serde_json::from_slice::<Response>(&out).map_err(|_| STATUS_MALFORMED)
    }

    /// Issue an HTTP GET request.
    pub fn get(url: &str) -> Result<Response, i32> {
        request("GET", url, None)
    }

    /// Issue an HTTP POST request with a string body.
    pub fn post(url: &str, body: &str) -> Result<Response, i32> {
        request("POST", url, Some(body))
    }

    /// Build the request JSON for the given parameters. Exposed for testing the
    /// marshalling without a live host.
    #[doc(hidden)]
    pub fn __request_json(method: &str, url: &str, body: Option<&str>) -> String {
        serde_json::json!({ "url": url, "method": method, "body": body }).to_string()
    }
}

// --- D-Bus -----------------------------------------------------------------

/// D-Bus helper. The host bridge is a permission-gated stub in daemon story
/// 6-3, so [`call`] returns `Err(-5)` (unimplemented) even when granted — plugins
/// should degrade gracefully.
pub mod dbus {
    use super::*;
    use alloc::string::String;
    use alloc::string::ToString;

    /// Invoke a D-Bus method. Returns the decoded JSON reply, or a status code.
    pub fn call(
        service: &str,
        path: &str,
        interface: &str,
        method: &str,
    ) -> Result<serde_json::Value, i32> {
        let req = serde_json::json!({
            "service": service,
            "path": path,
            "interface": interface,
            "method": method,
        });
        let bytes = serde_json::to_vec(&req).map_err(|_| STATUS_MALFORMED)?;
        let out = call_out_buffer(|optr, ocap| unsafe {
            sys::dbus_call(bytes.as_ptr() as i32, bytes.len() as i32, optr, ocap)
        })?;
        serde_json::from_slice(&out).map_err(|_| STATUS_MALFORMED)
    }

    /// Build the request JSON for the given parameters (testing helper).
    #[doc(hidden)]
    pub fn __request_json(service: &str, path: &str, interface: &str, method: &str) -> String {
        serde_json::json!({
            "service": service, "path": path, "interface": interface, "method": method,
        })
        .to_string()
    }
}

// --- plugin trait & entry point -------------------------------------------

/// The lifecycle hooks a plugin implements. All methods have no-op defaults, so
/// a plugin overrides only what it needs. Invoked by the exports that
/// [`plugin_entry!`] emits.
pub trait Plugin {
    /// Called once after instantiation, before `start`.
    fn init(&mut self) {}
    /// Called when the plugin becomes active.
    fn start(&mut self) {}
    /// Called on each daemon tick while running.
    fn tick(&mut self) {}
    /// Called with a serialized event payload the plugin subscribed to.
    fn handle_event(&mut self, _event: &serde_json::Value) {}
    /// Called when the plugin is stopped.
    fn stop(&mut self) {}
    /// Called just before the plugin is unloaded.
    fn unload(&mut self) {}
}

/// Read a `(ptr, len)` region the host wrote, returning it as bytes. Used by the
/// generated `handle_event` export. Safety: `ptr`/`len` must denote a valid
/// region the host allocated for this call.
#[doc(hidden)]
pub unsafe fn read_host_bytes(ptr: i32, len: i32) -> Vec<u8> {
    if ptr <= 0 || len <= 0 {
        return Vec::new();
    }
    core::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec()
}

/// Re-exports used by the SDK macros. Not part of the stable API.
#[doc(hidden)]
pub mod __rt {
    pub use crate::{alloc_impl, dealloc_impl, emit_activity, log_error, log_info, read_host_bytes};
    pub use alloc::format;
    pub use serde_json;
}

/// Re-exported so `emit_activity!(json!({...}))` works without a separate
/// `serde_json` dependency in the plugin crate.
pub use serde_json::json;

/// Generate the `#[no_mangle]` WASM exports (`alloc`, `dealloc`, `init`,
/// `start`, `tick`, `handle_event`, `stop`, `unload`) that bridge the host ABI
/// to a `Plugin` implementation. Pass a type that implements [`Plugin`] and is
/// constructible with `Default`.
///
/// ```ignore
/// plugin_entry!(MyPlugin);
/// ```
#[macro_export]
macro_rules! plugin_entry {
    ($plugin:ty) => {
        // A single global instance. Plugins are single-threaded in the sandbox.
        static mut __DYD_PLUGIN: ::core::option::Option<$plugin> = ::core::option::Option::None;

        #[allow(static_mut_refs)]
        unsafe fn __dyd_instance() -> &'static mut $plugin {
            if __DYD_PLUGIN.is_none() {
                __DYD_PLUGIN = ::core::option::Option::Some(<$plugin as ::core::default::Default>::default());
            }
            __DYD_PLUGIN.as_mut().unwrap()
        }

        #[no_mangle]
        pub extern "C" fn alloc(len: i32) -> i32 {
            $crate::__rt::alloc_impl(len)
        }

        #[no_mangle]
        pub extern "C" fn dealloc(ptr: i32, len: i32) {
            $crate::__rt::dealloc_impl(ptr, len)
        }

        #[no_mangle]
        pub extern "C" fn init() -> i32 {
            unsafe { $crate::Plugin::init(__dyd_instance()) };
            0
        }

        #[no_mangle]
        pub extern "C" fn start() -> i32 {
            unsafe { $crate::Plugin::start(__dyd_instance()) };
            0
        }

        #[no_mangle]
        pub extern "C" fn tick() -> i32 {
            unsafe { $crate::Plugin::tick(__dyd_instance()) };
            0
        }

        #[no_mangle]
        pub extern "C" fn stop() -> i32 {
            unsafe { $crate::Plugin::stop(__dyd_instance()) };
            0
        }

        #[no_mangle]
        pub extern "C" fn unload() -> i32 {
            unsafe { $crate::Plugin::unload(__dyd_instance()) };
            0
        }

        #[no_mangle]
        pub extern "C" fn handle_event(ptr: i32, len: i32) -> i32 {
            let bytes = unsafe { $crate::__rt::read_host_bytes(ptr, len) };
            match $crate::__rt::serde_json::from_slice(&bytes) {
                ::core::result::Result::Ok(value) => {
                    unsafe { $crate::Plugin::handle_event(__dyd_instance(), &value) };
                    0
                }
                ::core::result::Result::Err(_) => $crate::STATUS_MALFORMED,
            }
        }
    };
}

/// Log an informational message, `format!`-style.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::__rt::log_info(&$crate::__rt::format!($($arg)*))
    };
}

/// Log an error message, `format!`-style.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::__rt::log_error(&$crate::__rt::format!($($arg)*))
    };
}

/// Emit an activity from a [`serde_json::Value`] expression.
#[macro_export]
macro_rules! emit_activity {
    ($value:expr) => {
        $crate::__rt::emit_activity(&$value)
    };
}

/// Subscribe to a named daemon event.
#[macro_export]
macro_rules! subscribe_event {
    ($name:expr) => {
        $crate::subscribe_event($name)
    };
}

/// Convenient glob import for plugin authors.
pub mod prelude {
    pub use crate::{
        config, dbus, emit_activity, http, log_error, log_info, subscribe_event, Plugin,
    };
    pub use crate::{emit_activity as emit_activity_fn, json, plugin_entry};
    pub use serde_json::Value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn alloc_rejects_nonpositive_len() {
        // On a 64-bit host a real heap pointer does not fit in i32, so we only
        // assert the guard behavior here; the real alloc/dealloc roundtrip is
        // exercised on wasm32 (32-bit pointers) via the example plugins (6-6).
        assert_eq!(alloc_impl(0), 0);
        assert_eq!(alloc_impl(-5), 0);
        // dealloc guards are no-ops for null / non-positive len.
        dealloc_impl(0, 0);
        dealloc_impl(0, 16);
        dealloc_impl(1234, 0);
    }

    #[test]
    fn http_request_json_shape() {
        let s = http::__request_json("GET", "https://api.github.com/x", None);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["url"], "https://api.github.com/x");
        assert_eq!(v["method"], "GET");
        assert!(v["body"].is_null());

        let p = http::__request_json("POST", "https://x/y", Some("hello"));
        let v: serde_json::Value = serde_json::from_str(&p).unwrap();
        assert_eq!(v["method"], "POST");
        assert_eq!(v["body"], "hello");
    }

    #[test]
    fn dbus_request_json_shape() {
        let s = dbus::__request_json("org.x", "/p", "iface", "M");
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["service"], "org.x");
        assert_eq!(v["path"], "/p");
        assert_eq!(v["interface"], "iface");
        assert_eq!(v["method"], "M");
    }

    #[test]
    fn emit_activity_serializes() {
        // On host the emit_activity import is a no-op stub returning 0.
        let v = json!({ "text": "hi", "n": 3 });
        assert_eq!(emit_activity(&v), 0);
        assert_eq!(v["n"], 3);
        let _ = "unused".to_string();
    }
}
