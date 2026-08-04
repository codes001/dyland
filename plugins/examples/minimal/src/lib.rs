//! Minimal plugin used as a size-check fixture for the SDK (<500KB AC).
#![no_std]

use dyd_plugin_sdk::prelude::*;

#[derive(Default)]
struct Minimal;

impl Plugin for Minimal {
    fn start(&mut self) {
        log_info!("minimal plugin started");
    }

    fn tick(&mut self) {
        emit_activity!(json!({ "text": "tick" }));
    }
}

plugin_entry!(Minimal);
