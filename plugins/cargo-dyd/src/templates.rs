//! Text templates emitted by `cargo dyd init`.

/// `Cargo.toml` for a new plugin crate. The SDK is referenced by version; a
/// path override can be added by the author for local development.
pub fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
dyd-plugin-sdk = "0.1"

[profile.release]
# Small artifacts: strip, optimize for size, abort on panic.
opt-level = "z"
lto = true
strip = true
panic = "abort"
"#
    )
}

/// A starter `manifest.json` for the scaffolded plugin.
pub fn manifest_json(name: &str) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "name": "{name}",
  "version": "0.1.0",
  "author": "your name",
  "description": "A dyland plugin",
  "entry_point": "plugin.wasm",
  "permissions": [],
  "ui_components": ["expanded"],
  "dependencies": [],
  "sdk_version": "0.1"
}}
"#
    )
}

/// A minimal `src/lib.rs` demonstrating the SDK entry macro.
pub fn lib_rs(name: &str) -> String {
    format!(
        r#"//! {name} — a dyland plugin.
#![no_std]

use dyd_plugin_sdk::prelude::*;

#[derive(Default)]
struct {ident};

impl Plugin for {ident} {{
    fn start(&mut self) {{
        log_info!("{name} started");
    }}

    fn tick(&mut self) {{
        // Called each daemon tick while running. Emit activity as needed:
        emit_activity!(json!({{ "text": "hello from {name}" }}));
    }}
}}

plugin_entry!({ident});
"#,
        ident = to_ident(name)
    )
}

/// Turn a plugin name into a valid Rust type identifier (e.g. `docker-status`
/// -> `DockerStatus`).
fn to_ident(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            if upper {
                out.extend(ch.to_uppercase());
                upper = false;
            } else {
                out.push(ch);
            }
        } else {
            upper = true;
        }
    }
    if out.is_empty() || out.chars().next().unwrap().is_numeric() {
        out.insert(0, 'P');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ident_conversion() {
        assert_eq!(to_ident("docker-status"), "DockerStatus");
        assert_eq!(to_ident("my_plugin"), "MyPlugin");
        assert_eq!(to_ident("123abc"), "P123abc");
    }

    #[test]
    fn templates_reference_sdk() {
        assert!(lib_rs("x").contains("plugin_entry!"));
        assert!(cargo_toml("x").contains("dyd-plugin-sdk"));
        assert!(manifest_json("x").contains("\"entry_point\": \"plugin.wasm\""));
    }
}
