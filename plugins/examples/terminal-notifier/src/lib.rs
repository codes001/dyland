//! Reference plugin: **terminal command notifications**.
//!
//! Listens for shell command-completion events (delivered as
//! `org.freedesktop.Notifications` signals) and shows the command name, exit
//! code, and duration with a ✓ (exit 0) or ✗ (non-zero) glyph. Clicking the
//! activity focuses the originating terminal window.
//!
//! ## Wiring it up (see README)
//! Add a shell hook (bash `PROMPT_COMMAND` / zsh `precmd`) that posts a
//! notification carrying `command`, `exit_code`, `duration_ms`, and the terminal
//! window id. dyland routes matching notifications to this plugin's
//! `handle_event`.
//!
//! ## Host-support caveat (see README)
//! The daemon's inbound-event delivery (`on_event`) is a stub in story 6-3, so
//! completion events are not yet forwarded to plugins. The payload parser below
//! is fully unit-tested and works unchanged once event delivery lands. The
//! focus-terminal action additionally needs the `api:` command capability, which
//! is out of scope for this plugin's declared permissions.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};

use dyd_plugin_sdk::prelude::*;

#[derive(Default)]
struct TerminalNotifier;

/// A parsed shell command-completion event.
#[derive(Debug, PartialEq)]
struct Completion {
    command: String,
    exit_code: i64,
    duration_ms: u64,
    window_id: Option<String>,
}

/// Parse a completion payload. Requires at least `command`; missing exit code
/// defaults to 0, missing duration to 0.
fn parse_completion(payload: &serde_json::Value) -> Result<Completion, ()> {
    let command = payload.get("command").and_then(|v| v.as_str()).ok_or(())?;
    if command.is_empty() {
        return Err(());
    }
    Ok(Completion {
        command: command.to_string(),
        exit_code: payload.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0),
        duration_ms: payload
            .get("duration_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        window_id: payload
            .get("window_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

/// Human-friendly duration: `820ms`, `4.2s`, `1m03s`.
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        return format!("{ms}ms");
    }
    let secs = ms / 1000;
    if secs < 60 {
        let tenths = (ms % 1000) / 100;
        return format!("{secs}.{tenths}s");
    }
    format!("{}m{:02}s", secs / 60, secs % 60)
}

/// Build the activity for a completion, with a success/failure glyph and a
/// focus-terminal action when a window id is known.
fn completion_activity(c: &Completion) -> Value {
    let ok = c.exit_code == 0;
    let glyph = if ok { "✓" } else { "✗" };
    let text = format!("{glyph} {} ({})", c.command, format_duration(c.duration_ms));
    let mut activity = json!({
        "text": text,
        "icon": if ok { "check" } else { "cross" },
        "severity": if ok { "info" } else { "error" },
        "expanded": {
            "command": c.command,
            "exit_code": c.exit_code,
            "duration": format_duration(c.duration_ms),
        },
    });
    if let Some(id) = &c.window_id {
        // Consumed by the daemon when the activity is clicked (needs `api:`).
        activity["action"] = json!({ "type": "focus_window", "window_id": id });
    }
    activity
}

impl Plugin for TerminalNotifier {
    fn start(&mut self) {
        log_info!("terminal-notifier started");
        // Subscribe to shell completion notifications routed by the daemon.
        subscribe_event!("org.freedesktop.Notifications");
    }

    fn handle_event(&mut self, event: &serde_json::Value) {
        match parse_completion(event) {
            Ok(c) => {
                emit_activity!(completion_activity(&c));
            }
            Err(()) => {
                log_error!("terminal-notifier: unrecognized completion payload");
            }
        }
    }
}

plugin_entry!(TerminalNotifier);

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(cmd: &str, exit: i64, dur: u64) -> serde_json::Value {
        json!({ "command": cmd, "exit_code": exit, "duration_ms": dur, "window_id": "0x1" })
    }

    #[test]
    fn parses_command_exit_duration() {
        let c = parse_completion(&payload("cargo build", 0, 4200)).unwrap();
        assert_eq!(c.command, "cargo build");
        assert_eq!(c.exit_code, 0);
        assert_eq!(c.duration_ms, 4200);
        assert_eq!(c.window_id.as_deref(), Some("0x1"));
    }

    #[test]
    fn missing_fields_default_but_command_required() {
        let c = parse_completion(&json!({ "command": "ls" })).unwrap();
        assert_eq!((c.exit_code, c.duration_ms), (0, 0));
        assert!(parse_completion(&json!({ "exit_code": 1 })).is_err());
        assert!(parse_completion(&json!({ "command": "" })).is_err());
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration(820), "820ms");
        assert_eq!(format_duration(4200), "4.2s");
        assert_eq!(format_duration(63000), "1m03s");
    }

    #[test]
    fn success_uses_checkmark() {
        let a = completion_activity(&parse_completion(&payload("make", 0, 1500)).unwrap());
        assert_eq!(a["text"], "✓ make (1.5s)");
        assert_eq!(a["severity"], "info");
        assert_eq!(a["action"]["type"], "focus_window");
    }

    #[test]
    fn failure_uses_cross_and_error_severity() {
        let a = completion_activity(&parse_completion(&payload("test", 1, 900)).unwrap());
        assert_eq!(a["text"], "✗ test (900ms)");
        assert_eq!(a["severity"], "error");
    }
}
