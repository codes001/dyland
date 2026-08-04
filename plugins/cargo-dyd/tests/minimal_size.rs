//! AC7 size gate: the SDK + a trivial plugin must compile to a small wasm.
//!
//! This is an integration test (not a unit test) because it invokes a real
//! `cargo build --target wasm32-wasip1` on the `minimal` example fixture. When
//! the wasm target is not installed the test reports the reason and returns
//! rather than failing — CI without the target can still run the rest of the
//! suite. Where the target *is* installed (the default dev/CI setup) this
//! genuinely builds the artifact and enforces the 500 KB budget.

use std::path::PathBuf;

use cargo_dyd::{build, wasm_target_installed};

/// Ceiling from story 6-4 AC7: a minimal plugin stays well under 500 KB.
const MAX_WASM_BYTES: u64 = 500 * 1024;

fn minimal_example_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = plugins/cargo-dyd; the fixture lives at
    // plugins/examples/minimal.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("minimal")
}

#[test]
fn minimal_plugin_builds_under_size_budget() {
    if !wasm_target_installed() {
        eprintln!(
            "skipping: wasm32-wasip1 target not installed \
             (run `rustup target add wasm32-wasip1` to enable this check)"
        );
        return;
    }

    let dir = minimal_example_dir();
    assert!(
        dir.join("Cargo.toml").exists(),
        "minimal example fixture missing at {}",
        dir.display()
    );

    let wasm = build(&dir).expect("minimal plugin should build for wasm32-wasip1");
    let size = std::fs::metadata(&wasm)
        .expect("built wasm artifact should be stat-able")
        .len();

    assert!(
        size < MAX_WASM_BYTES,
        "minimal plugin wasm is {size} bytes, exceeds {MAX_WASM_BYTES} byte budget"
    );
}
