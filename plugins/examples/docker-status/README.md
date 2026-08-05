# docker-status

A reference [dyland](../../..) plugin that shows Docker container status in the
Dynamic Island: `running/total` in the pill, and a per-container list
(name / state / uptime) in the expanded view. Polls every 10 seconds.

## What it demonstrates

- The `Plugin` lifecycle (`start`, `tick`) from `dyd-plugin-sdk`.
- A tick-counted timer (the daemon ticks at 100ms; the plugin polls every 100th).
- The host `http_request` capability via `http::get`.
- Robust JSON parsing with a fully degraded fallback state.

## Permissions

```json
"permissions": ["unix_socket:/var/run/docker.sock"]
```

The plugin issues `GET /v1.41/containers/json?all=1` and parses Docker's standard
container-list JSON (`Names`, `State`, `Status`).

## ⚠️ Host-support caveat

The daemon's `http_request` host function (story 6-3) currently speaks **HTTP over
TCP** and does **not** yet bridge the `unix_socket:` permission to an
HTTP-over-unix-socket transport. Until that bridge lands:

- `http::get` returns an error,
- the plugin renders the **"Docker not available"** state (the AC2 degraded path),
- **no crash** — the container-parsing logic is exercised and unit-tested regardless.

When the host adds unix-socket HTTP, this plugin works end-to-end with no changes.

## States

| Condition | Pill |
|---|---|
| Docker reachable | `2/3 running` + container list |
| Docker down / transport absent | `Docker not available` |
| Permission denied (`-1`) | `Docker not available` (error logged) |

## Build & install

```bash
# from this directory
cargo dyd build       # cargo build --release --target wasm32-wasip1
cargo dyd package     # produces dyd-docker-status-0.1.0.dyd
cargo dyd install     # unzips into $XDG_DATA_HOME/dyd/plugins/docker-status/
```

Or point the daemon at a built artifact directly with `DYD_PLUGINS_DIR`.

## Test

```bash
cargo test                                  # container-parsing unit tests
cargo build --release --target wasm32-wasip1  # compile check for wasm
```
