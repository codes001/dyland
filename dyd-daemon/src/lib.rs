pub mod health;
pub mod ipc;
pub mod lifecycle;
pub mod queue;
pub mod signals;
pub mod sources;

pub use health::HealthCheck;
pub use lifecycle::DaemonState;

use sources::fullscreen::{poll_fullscreen, FullscreenState};
use sources::media::MediaSource;
use sources::notifications::NotificationSource;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::OnceLock;
use tokio::sync::watch;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing_subscriber::EnvFilter;

static LOG_INIT: OnceLock<()> = OnceLock::new();

pub struct Daemon {
    state: DaemonState,
    shutdown_tx: watch::Sender<()>,
    shutdown_rx: watch::Receiver<()>,
    dbus_conn: Option<zbus::Connection>,
    health: Arc<HealthCheck>,
    connected_clients: Arc<AtomicU32>,
    start_time: tokio::time::Instant,
    ipc_server_handle: Option<JoinHandle<()>>,
    fullscreen_state: Arc<FullscreenState>,
    fullscreen_tx: watch::Sender<bool>,
    fullscreen_rx: watch::Receiver<bool>,
    activity_queue: Arc<tokio::sync::Mutex<queue::ActivityQueue>>,
    media_source: Option<Arc<MediaSource>>,
    notification_source: Option<Arc<NotificationSource>>,
}

impl Daemon {
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let connected_clients = Arc::new(AtomicU32::new(0));
        let health = Arc::new(HealthCheck::new(connected_clients.clone()));
        let (fullscreen_tx, fullscreen_rx) = watch::channel(false);
        let fullscreen_state = Arc::new(FullscreenState::new());
        let activity_queue = Arc::new(tokio::sync::Mutex::new(queue::ActivityQueue::new()));

        Self {
            state: DaemonState::Starting,
            shutdown_tx,
            shutdown_rx,
            dbus_conn: None,
            health,
            connected_clients,
            start_time: tokio::time::Instant::now(),
            ipc_server_handle: None,
            fullscreen_state,
            fullscreen_tx,
            fullscreen_rx,
            activity_queue,
            media_source: None,
            notification_source: None,
        }
    }

    pub async fn run(&mut self) {
        self.setup_logging();
        tracing::info!(pid = std::process::id(), "dyd-daemon starting");

        self.connect_dbus().await;
        self.init_media_source().await;
        self.init_notification_source().await;
        self.start_ipc_server().await;
        self.start_fullscreen_poller().await;
        self.start_media_poller().await;
        self.start_notification_listener().await;
        self.state = DaemonState::Running;
        self.health.set_ready();
        self.start_time = tokio::time::Instant::now();
        tracing::info!("daemon ready");

        let mut sigterm = signals::sigterm_signal();
        let mut sigint = signals::sigint_signal();

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, initiating shutdown");
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT, initiating shutdown");
            }
            _ = self.shutdown_rx.changed() => {}
        }

        self.shutdown().await;
    }

    fn setup_logging(&self) {
        LOG_INIT.get_or_init(|| {
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::try_from_env("DYD_LOG")
                        .unwrap_or_else(|_| EnvFilter::new("info")),
                )
                .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
                .with_target(true)
                .with_line_number(true)
                .init();
        });
    }

    async fn connect_dbus(&mut self) {
        match zbus::Connection::session().await {
            Ok(conn) => {
                let unique_name = conn.unique_name().map_or("unknown", |v| v).to_string();
                tracing::info!(dbus.unique_name = %unique_name, "D-Bus session connection established");
                self.dbus_conn = Some(conn);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to connect to D-Bus session bus. Continuing without D-Bus."
                );
            }
        }
    }

    async fn init_media_source(&mut self) {
        let mut source = MediaSource::new();
        if let Some(conn) = self.dbus_conn.as_ref() {
            source.set_dbus_connection(conn.clone());
        }
        self.media_source = Some(Arc::new(source));
    }

    async fn start_ipc_server(&mut self) {
        let health = self.health.clone();
        let clients = self.connected_clients.clone();
        let shutdown_rx = self.shutdown_rx.clone();
        let (ready_tx, ready_rx) = oneshot::channel::<()>();
        let media_broadcast = self.media_source.as_ref().map(|m| m.event_broadcast());
        let media_source = self.media_source.clone();
        let notification_broadcast = self.notification_source.as_ref().map(|n| n.event_broadcast());
        let notification_source = self.notification_source.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = ipc::server::start(health, clients, shutdown_rx, ready_tx, media_broadcast, media_source, notification_broadcast, notification_source).await {
                tracing::error!(error = %e, "IPC server failed");
            }
        });

        self.ipc_server_handle = Some(handle);

        tokio::select! {
            _ = ready_rx => {
                tracing::info!("IPC server ready");
            }
            _ = self.shutdown_rx.changed() => {
                tracing::warn!("shutdown requested before IPC server was ready");
            }
        }
    }

    async fn start_fullscreen_poller(&mut self) {
        let state = self.fullscreen_state.clone();
        let tx = self.fullscreen_tx.clone();
        let shutdown_rx = self.shutdown_rx.clone();

        tokio::spawn(async move {
            poll_fullscreen(state, tx, shutdown_rx).await;
        });
        tracing::info!("fullscreen poller started");
    }

    async fn start_media_poller(&mut self) {
        if let Some(source) = self.media_source.as_ref() {
            let shutdown_rx = self.shutdown_rx.clone();
            let source = source.clone();
            tokio::spawn(async move {
                source.start_polling(shutdown_rx).await;
            });
            tracing::info!("media poller started");
        }
    }

    async fn init_notification_source(&mut self) {
        let mut source = NotificationSource::new();
        if let Some(conn) = self.dbus_conn.as_ref() {
            source.set_dbus_connection(conn.clone());
        }
        self.notification_source = Some(Arc::new(source));
        tracing::info!("notification source initialized");
    }

    async fn start_notification_listener(&mut self) {
        if let Some(source) = self.notification_source.as_ref() {
            let shutdown_rx = self.shutdown_rx.clone();
            let source = source.clone();
            tokio::spawn(async move {
                source.start_listening(shutdown_rx).await;
            });
            tracing::info!("notification listener started");
        }
    }

    pub fn activity_queue(&self) -> Arc<tokio::sync::Mutex<queue::ActivityQueue>> {
        self.activity_queue.clone()
    }

    pub fn fullscreen_receiver(&self) -> watch::Receiver<bool> {
        self.fullscreen_rx.clone()
    }

    pub fn fullscreen_state_ref(&self) -> Arc<FullscreenState> {
        self.fullscreen_state.clone()
    }

    pub fn media_source_ref(&self) -> Option<Arc<MediaSource>> {
        self.media_source.clone()
    }

    pub fn notification_source_ref(&self) -> Option<Arc<NotificationSource>> {
        self.notification_source.clone()
    }

    async fn shutdown(&mut self) {
        self.state = DaemonState::ShuttingDown;
        self.health.set_shutting_down();
        tracing::info!("daemon shutting down");

        let _ = self.shutdown_tx.send(());

        if let Some(handle) = self.ipc_server_handle.take() {
            handle.abort();
        }

        if let Some(conn) = self.dbus_conn.take() {
            drop(conn);
            tracing::info!("D-Bus connection closed");
        }

        self.activity_queue.lock().await.clear();

        let _ = std::fs::remove_file("/tmp/dyd-daemon.sock");

        self.state = DaemonState::Stopped;
        tracing::info!("daemon stopped");
    }

    pub fn shutdown_handle(&self) -> watch::Receiver<()> {
        self.shutdown_rx.clone()
    }

    pub fn health_check(&self) -> Arc<HealthCheck> {
        self.health.clone()
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for Daemon {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = std::fs::remove_file("/tmp/dyd-daemon.sock");
    }
}
