//! Reference plugin: **Docker container status**.
//!
//! Every 10 seconds it asks the daemon to `GET /containers/json?all=1` on the
//! Docker socket, parses the running/total counts and a short container list,
//! and emits an activity for the pill + expanded view.
//!
//! ## Host-support caveat (see README)
//! The daemon's `http_request` host function (story 6-3) speaks HTTP over TCP;
//! it does not yet bridge the `unix_socket:` permission to an HTTP-over-unix
//! transport. Until it does, `http::get` fails and the plugin renders the
//! **"Docker not available"** state — which is exactly the AC2 degraded path.
//! The container-parsing logic below is fully unit-tested regardless.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use dyd_plugin_sdk::prelude::*;

/// Docker exposes its API on this host when tunnelled through the socket. The
/// authority is ignored by a unix-socket transport but keeps the URL valid.
const CONTAINERS_URL: &str = "http://localhost/v1.41/containers/json?all=1";

/// Ticks between polls. The daemon ticks every 100ms, so 100 ticks ≈ 10s.
const POLL_EVERY_TICKS: u32 = 100;

#[derive(Default)]
struct DockerStatus {
    ticks: u32,
}

/// One container row for the expanded view.
#[derive(Debug, PartialEq)]
struct Container {
    name: String,
    state: String,
    status: String,
}

/// Parsed summary of `GET /containers/json?all=1`.
#[derive(Debug, PartialEq)]
struct Summary {
    running: usize,
    total: usize,
    containers: Vec<Container>,
}

/// Strip Docker's leading `/` from the first entry of a `Names` array.
fn primary_name(names: Option<&serde_json::Value>) -> String {
    names
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('/').to_string())
        .unwrap_or_else(|| "<unnamed>".to_string())
}

/// Parse the Docker `containers/json` array into a [`Summary`].
fn parse_containers(body: &str) -> Result<Summary, ()> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|_| ())?;
    let arr = value.as_array().ok_or(())?;
    let mut containers = Vec::new();
    let mut running = 0usize;
    for item in arr {
        let state = item
            .get("State")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        if state == "running" {
            running += 1;
        }
        containers.push(Container {
            name: primary_name(item.get("Names")),
            state,
            status: item
                .get("Status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    Ok(Summary {
        total: containers.len(),
        running,
        containers,
    })
}

/// Build the activity payload for a parsed summary.
fn summary_activity(s: &Summary) -> Value {
    let list: Vec<Value> = s
        .containers
        .iter()
        .map(|c| json!({ "name": c.name, "state": c.state, "status": c.status }))
        .collect();
    json!({
        "text": format!("{}/{} running", s.running, s.total),
        "icon": "docker",
        "expanded": { "containers": list },
    })
}

/// Activity shown when Docker can't be reached.
fn unavailable_activity() -> Value {
    json!({ "text": "Docker not available", "icon": "docker", "state": "unavailable" })
}

impl Plugin for DockerStatus {
    fn start(&mut self) {
        log_info!("docker-status started");
        self.poll();
    }

    fn tick(&mut self) {
        self.ticks += 1;
        if self.ticks >= POLL_EVERY_TICKS {
            self.ticks = 0;
            self.poll();
        }
    }
}

impl DockerStatus {
    fn poll(&self) {
        match http::get(CONTAINERS_URL) {
            Ok(resp) if resp.status == 200 => match parse_containers(&resp.body) {
                Ok(summary) => {
                    emit_activity!(summary_activity(&summary));
                }
                Err(()) => {
                    log_error!("docker-status: malformed container list");
                    emit_activity!(unavailable_activity());
                }
            },
            Ok(resp) => {
                log_error!("docker-status: docker returned HTTP {}", resp.status);
                emit_activity!(unavailable_activity());
            }
            Err(code) => {
                // -1 (denied) surfaces the permission error; anything else means
                // the socket is unreachable (docker down or transport absent).
                log_error!("docker-status: http_request failed ({code})");
                emit_activity!(unavailable_activity());
            }
        }
    }
}

plugin_entry!(DockerStatus);

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
        {"Names":["/web"],"State":"running","Status":"Up 3 hours"},
        {"Names":["/db"],"State":"running","Status":"Up 3 hours (healthy)"},
        {"Names":["/worker"],"State":"exited","Status":"Exited (0) 5 minutes ago"}
    ]"#;

    #[test]
    fn parses_running_and_total() {
        let s = parse_containers(SAMPLE).unwrap();
        assert_eq!(s.total, 3);
        assert_eq!(s.running, 2);
    }

    #[test]
    fn parses_container_list_with_clean_names() {
        let s = parse_containers(SAMPLE).unwrap();
        assert_eq!(s.containers[0].name, "web");
        assert_eq!(s.containers[2].state, "exited");
        assert_eq!(s.containers[1].status, "Up 3 hours (healthy)");
    }

    #[test]
    fn empty_list_is_zero_zero() {
        let s = parse_containers("[]").unwrap();
        assert_eq!((s.running, s.total), (0, 0));
    }

    #[test]
    fn malformed_json_errors() {
        assert!(parse_containers("not json").is_err());
        assert!(parse_containers("{}").is_err());
    }

    #[test]
    fn activity_reports_counts() {
        let s = parse_containers(SAMPLE).unwrap();
        let a = summary_activity(&s);
        assert_eq!(a["text"], "2/3 running");
        assert_eq!(a["expanded"]["containers"].as_array().unwrap().len(), 3);
    }
}
