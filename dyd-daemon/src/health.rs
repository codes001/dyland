use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::lifecycle::DaemonState;

const STATE_STARTING: u8 = 0;
const STATE_RUNNING: u8 = 1;
const STATE_SHUTTING_DOWN: u8 = 2;
const STATE_STOPPED: u8 = 3;

pub struct HealthCheck {
    start_time: Instant,
    state: AtomicU8,
    connected_clients: Arc<AtomicU32>,
    version: &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub uptime_secs: u64,
    pub version: String,
    pub connected_clients: u32,
}

impl HealthCheck {
    pub fn new(connected_clients: Arc<AtomicU32>) -> Self {
        Self {
            start_time: Instant::now(),
            state: AtomicU8::new(STATE_STARTING),
            connected_clients,
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    pub fn set_ready(&self) {
        self.state.store(STATE_RUNNING, Ordering::Release);
    }

    pub fn set_shutting_down(&self) {
        self.state.store(STATE_SHUTTING_DOWN, Ordering::Release);
    }

    pub fn status(&self) -> HealthStatus {
        let state = self.state.load(Ordering::Acquire);
        let status = match state {
            STATE_SHUTTING_DOWN => "shutting_down",
            STATE_RUNNING => "ok",
            _ => "degraded",
        };

        HealthStatus {
            status: status.to_string(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            version: self.version.to_string(),
            connected_clients: self.connected_clients.load(Ordering::Relaxed),
        }
    }

    pub fn daemon_state(&self) -> DaemonState {
        match self.state.load(Ordering::Acquire) {
            STATE_RUNNING => DaemonState::Running,
            STATE_SHUTTING_DOWN => DaemonState::ShuttingDown,
            STATE_STOPPED => DaemonState::Stopped,
            _ => DaemonState::Starting,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_status_is_degraded() {
        let clients = Arc::new(AtomicU32::new(0));
        let health = HealthCheck::new(clients);
        let status = health.status();
        assert_eq!(status.status, "degraded");
    }

    #[test]
    fn test_ready_status_is_ok() {
        let clients = Arc::new(AtomicU32::new(0));
        let health = HealthCheck::new(clients);
        health.set_ready();
        let status = health.status();
        assert_eq!(status.status, "ok");
    }

    #[test]
    fn test_shutting_down_status() {
        let clients = Arc::new(AtomicU32::new(0));
        let health = HealthCheck::new(clients);
        health.set_ready();
        health.set_shutting_down();
        let status = health.status();
        assert_eq!(status.status, "shutting_down");
    }

    #[test]
    fn test_uptime_increases() {
        let clients = Arc::new(AtomicU32::new(0));
        let health = HealthCheck::new(clients);
        health.set_ready();
        let status1 = health.status();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let status2 = health.status();
        assert!(status2.uptime_secs >= status1.uptime_secs);
    }

    #[test]
    fn test_connected_clients_reported() {
        let clients = Arc::new(AtomicU32::new(5));
        let health = HealthCheck::new(clients);
        health.set_ready();
        let status = health.status();
        assert_eq!(status.connected_clients, 5);
    }

    #[test]
    fn test_version_is_set() {
        let clients = Arc::new(AtomicU32::new(0));
        let health = HealthCheck::new(clients);
        let status = health.status();
        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
    }
}
