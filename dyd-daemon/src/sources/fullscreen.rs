use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::watch;
use tokio::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct FullscreenState {
    is_fullscreen: Arc<AtomicBool>,
    monitor_count: Arc<std::sync::atomic::AtomicU32>,
}

impl FullscreenState {
    pub fn new() -> Self {
        Self {
            is_fullscreen: Arc::new(AtomicBool::new(false)),
            monitor_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        self.is_fullscreen.load(Ordering::Relaxed)
    }

    pub fn set_fullscreen(&self, fullscreen: bool) {
        self.is_fullscreen.store(fullscreen, Ordering::Relaxed);
    }

    pub fn increment_monitor_count(&self) -> u32 {
        self.monitor_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn decrement_monitor_count(&self) -> u32 {
        let prev = self.monitor_count.fetch_sub(1, Ordering::Relaxed);
        if prev == 0 {
            self.monitor_count.store(0, Ordering::Relaxed);
            0
        } else {
            prev - 1
        }
    }

    pub fn monitor_count(&self) -> u32 {
        self.monitor_count.load(Ordering::Relaxed)
    }
}

impl Default for FullscreenState {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn poll_fullscreen(
    fullscreen_state: FullscreenState,
    tx: watch::Sender<bool>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    let mut last_fullscreen = false;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                tracing::info!("fullscreen poller shutting down");
                break;
            }
            _ = tokio::time::sleep(POLL_INTERVAL) => {
                let current = poll_dbus_fullscreen().await;
                let is_full = current > 0;

                if is_full != last_fullscreen {
                    last_fullscreen = is_full;
                    fullscreen_state.set_fullscreen(is_full);
                    let _ = tx.send(is_full);
                    tracing::info!(fullscreen = is_full, "fullscreen state changed");
                }
            }
        }
    }
}

async fn poll_dbus_fullscreen() -> u32 {
    match zbus::Connection::session().await {
        Ok(conn) => {
            let proxy = match conn
                .call_method(
                    Some("org.gnome.Shell"),
                    "/org/gnome/Shell",
                    Some("org.gnome.Shell.Introspect"),
                    "GetWindows",
                    &(),
                )
                .await
            {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::debug!(error = %e, "gnome shell introspection not available");
                    return 0;
                }
            };

            let windows: Vec<serde_json::Value> = match proxy.body().deserialize() {
                Ok(w) => w,
                Err(_) => return 0,
            };

            windows
                .iter()
                .filter(|w| {
                    w.get("fullscreen").and_then(|v| v.as_bool()).unwrap_or(false)
                })
                .count() as u32
        }
        Err(_) => 0,
    }
}
