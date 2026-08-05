# terminal-notifier

A reference [dyland](../../..) plugin that shows the result of long-running
terminal commands in the Dynamic Island: `✓ cargo build (4.2s)` on success,
`✗ cargo test (900ms)` on failure. Clicking the activity focuses the terminal
that ran the command.

## What it demonstrates

- Event-driven plugins: `subscribe_event!` + `handle_event`.
- Parsing an inbound event payload into a typed struct.
- A success/failure glyph, human-friendly duration formatting, and a
  click-to-focus action attached to the activity.

## Permissions

```json
"permissions": ["dbus:org.freedesktop.Notifications"]
```

## Wiring it up

Add a shell hook that posts a completion event when a command finishes. Example
(bash):

```bash
__dyd_notify() {
  local exit=$?
  local dur_ms=$(( (${EPOCHREALTIME/./} - ${__dyd_start:-0}) / 1000 ))
  notify-send -a dyd-term \
    -h "string:command:${__dyd_last_cmd}" \
    -h "int:exit_code:${exit}" \
    -h "int:duration_ms:${dur_ms}" \
    -h "string:window_id:${WINDOWID}" \
    "command finished"
}
trap '__dyd_start=${EPOCHREALTIME/./}; __dyd_last_cmd=$BASH_COMMAND' DEBUG
PROMPT_COMMAND=__dyd_notify
```

dyland routes matching `org.freedesktop.Notifications` events to the plugin's
`handle_event`, which expects a JSON payload:

```json
{ "command": "cargo build", "exit_code": 0, "duration_ms": 4200, "window_id": "0x1" }
```

## States

| Exit code | Pill |
|---|---|
| `0` | `✓ <command> (<duration>)` — info severity |
| non-zero | `✗ <command> (<duration>)` — error severity |

## ⚠️ Host-support caveat

Two host capabilities are still stubs in story 6-3:

1. **Inbound event delivery** (`on_event`) is not yet forwarded to plugins, so
   completion events don't reach `handle_event` at runtime. The payload parser is
   fully unit-tested and works unchanged once delivery lands.
2. **Focus-terminal on click** needs the `api:` command capability to send a
   window-activate command; this plugin declares only the notifications
   permission, so the `action` field is emitted for the daemon to honor later.

Neither gap causes a crash — the plugin degrades to doing nothing until the host
support arrives.

## Build & test

```bash
cargo dyd build                              # release wasm32-wasip1 build
cargo test                                   # payload + duration unit tests
cargo build --release --target wasm32-wasip1   # compile check
```
