pub mod health;
pub mod ipc;
pub mod lifecycle;
pub mod plugins;
pub mod queue;
pub mod signals;
pub mod sources;

pub use health::HealthCheck;
pub use lifecycle::DaemonState;

use sources::battery::BatterySource;
use sources::bluetooth::BluetoothSource;
use sources::brightness::BrightnessSource;
use sources::fullscreen::{poll_fullscreen, FullscreenState};
use sources::media::MediaSource;
use sources::notifications::NotificationSource;
use sources::volume::VolumeSource;
use sources::wifi::WifiSource;

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing_subscriber::EnvFilter;

use plugins::manager::PluginManager;
use plugins::host::{PluginExecutor, StubExecutor};
use plugins::runtime::{HostContext, WasmExecutor};

static LOG_INIT: OnceLock<()> = OnceLock::new();

pub struct Daemon {
    state: DaemonState,
    shutdown_tx: watch::Sender<()>,
    shutdown_rx: watch::Receiver<()>,
    dbus_conn: Option<zbus::Connection>,
    dbus_system_conn: Option<zbus::Connection>,
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
    battery_source: Option<Arc<BatterySource>>,
    wifi_source: Option<Arc<WifiSource>>,
    bluetooth_source: Option<Arc<BluetoothSource>>,
    volume_source: Option<Arc<VolumeSource>>,
    brightness_source: Option<Arc<BrightnessSource>>,
    plugin_manager: Arc<PluginManager>,
}

impl Daemon {
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let connected_clients = Arc::new(AtomicU32::new(0));
        let health = Arc::new(HealthCheck::new(connected_clients.clone()));
        let (fullscreen_tx, fullscreen_rx) = watch::channel(false);
        let fullscreen_state = Arc::new(FullscreenState::new());
        let activity_queue = Arc::new(tokio::sync::Mutex::new(queue::ActivityQueue::new()));

        // Build the shared host context and WASM executor. If the engine fails
        // to initialize (should not happen), fall back to the no-op stub so the
        // daemon still runs without plugins.
        let (plugin_event_tx, _) = tokio::sync::broadcast::channel(64);
        let host_ctx = Arc::new(HostContext::new(
            plugin_event_tx.clone(),
            tokio::runtime::Handle::current(),
        ));
        let executor: Arc<dyn PluginExecutor> = match WasmExecutor::new(host_ctx.clone()) {
            Ok(exec) => Arc::new(exec),
            Err(e) => {
                tracing::error!(error = %e, "wasm executor init failed; plugins disabled");
                Arc::new(StubExecutor)
            }
        };
        let plugin_manager = Arc::new(PluginManager::from_parts(
            executor,
            plugin_event_tx,
            host_ctx,
        ));

        Self {
            state: DaemonState::Starting,
            shutdown_tx,
            shutdown_rx,
            dbus_conn: None,
            dbus_system_conn: None,
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
            battery_source: None,
            wifi_source: None,
            bluetooth_source: None,
            volume_source: None,
            brightness_source: None,
            plugin_manager,
        }
    }

    pub async fn run(&mut self) {
        self.setup_logging();
        tracing::info!(pid = std::process::id(), "dyd-daemon starting");

        self.connect_dbus().await;
        self.init_media_source().await;
        self.init_notification_source().await;
        self.init_battery_source().await;
        self.init_wifi_source().await;
        self.init_bluetooth_source().await;
        self.init_volume_source().await;
        self.init_brightness_source().await;
        self.start_ipc_server().await;
        self.start_fullscreen_poller().await;
        self.start_media_poller().await;
        self.start_notification_listener().await;
        self.start_battery_listener().await;
        self.start_wifi_poller().await;
        self.start_bluetooth_poller().await;
        self.start_volume_poller().await;
        self.start_brightness_poller().await;
        self.init_plugins().await;
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
                    EnvFilter::try_from_env("DYD_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
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

        match zbus::Connection::system().await {
            Ok(conn) => {
                let unique_name = conn.unique_name().map_or("unknown", |v| v).to_string();
                tracing::info!(dbus.unique_name = %unique_name, "D-Bus system connection established");
                self.dbus_system_conn = Some(conn);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to connect to D-Bus system bus. Continuing without D-Bus."
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
        let sources = ipc::context::SourceContext {
            media_broadcast: self.media_source.as_ref().map(|m| m.event_broadcast()),
            notification_broadcast: self
                .notification_source
                .as_ref()
                .map(|n| n.event_broadcast()),
            battery_broadcast: self.battery_source.as_ref().map(|b| b.event_broadcast()),
            wifi_broadcast: self.wifi_source.as_ref().map(|w| w.event_broadcast()),
            bluetooth_broadcast: self.bluetooth_source.as_ref().map(|b| b.event_broadcast()),
            volume_broadcast: self.volume_source.as_ref().map(|v| v.event_broadcast()),
            brightness_broadcast: self.brightness_source.as_ref().map(|b| b.event_broadcast()),
            media_source: self.media_source.clone(),
            notification_source: self.notification_source.clone(),
            battery_source: self.battery_source.clone(),
            wifi_source: self.wifi_source.clone(),
            bluetooth_source: self.bluetooth_source.clone(),
            volume_source: self.volume_source.clone(),
            brightness_source: self.brightness_source.clone(),
            plugin_manager: Some(self.plugin_manager.clone()),
            plugin_broadcast: Some(self.plugin_manager.event_broadcast()),
        };

        let handle = tokio::spawn(async move {
            if let Err(e) =
                ipc::server::start(health, clients, shutdown_rx, ready_tx, sources).await
            {
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

    async fn init_battery_source(&mut self) {
        let mut source = BatterySource::new();
        if let Some(conn) = self.dbus_system_conn.as_ref() {
            source.set_dbus_connection(conn.clone());
        }
        self.battery_source = Some(Arc::new(source));
        tracing::info!("battery source initialized");
    }

    async fn start_battery_listener(&mut self) {
        if let Some(source) = self.battery_source.as_ref() {
            let shutdown_rx = self.shutdown_rx.clone();
            let source = source.clone();
            tokio::spawn(async move {
                source.start_listening(shutdown_rx).await;
            });
            tracing::info!("battery listener started");
        }
    }

    async fn init_wifi_source(&mut self) {
        let mut source = WifiSource::new();
        if let Some(conn) = self.dbus_system_conn.as_ref() {
            source.set_dbus_connection(conn.clone());
        }
        self.wifi_source = Some(Arc::new(source));
        tracing::info!("wifi source initialized");
    }

    async fn start_wifi_poller(&mut self) {
        if let Some(source) = self.wifi_source.as_ref() {
            let shutdown_rx = self.shutdown_rx.clone();
            let source = source.clone();
            tokio::spawn(async move {
                source.start_polling(shutdown_rx).await;
            });
            tracing::info!("wifi poller started");
        }
    }

    async fn init_bluetooth_source(&mut self) {
        let mut source = BluetoothSource::new();
        if let Some(conn) = self.dbus_system_conn.as_ref() {
            source.set_dbus_connection(conn.clone());
        }
        self.bluetooth_source = Some(Arc::new(source));
        tracing::info!("bluetooth source initialized");
    }

    async fn start_bluetooth_poller(&mut self) {
        if let Some(source) = self.bluetooth_source.as_ref() {
            let shutdown_rx = self.shutdown_rx.clone();
            let source = source.clone();
            tokio::spawn(async move {
                source.start_polling(shutdown_rx).await;
            });
            tracing::info!("bluetooth poller started");
        }
    }

    async fn init_volume_source(&mut self) {
        let source = VolumeSource::new();
        self.volume_source = Some(Arc::new(source));
        tracing::info!("volume source initialized");
    }

    async fn start_volume_poller(&mut self) {
        if let Some(source) = self.volume_source.as_ref() {
            let shutdown_rx = self.shutdown_rx.clone();
            let source = source.clone();
            tokio::spawn(async move {
                source.start_polling(shutdown_rx).await;
            });
            tracing::info!("volume poller started");
        }
    }

    async fn init_brightness_source(&mut self) {
        let mut source = BrightnessSource::new();
        if let Some(conn) = self.dbus_conn.as_ref() {
            source.set_dbus_connection(conn.clone());
        }
        self.brightness_source = Some(Arc::new(source));
        tracing::info!("brightness source initialized");
    }

    async fn start_brightness_poller(&mut self) {
        if let Some(source) = self.brightness_source.as_ref() {
            let shutdown_rx = self.shutdown_rx.clone();
            let source = source.clone();
            tokio::spawn(async move {
                source.start_polling(shutdown_rx).await;
            });
            tracing::info!("brightness poller started");
        }
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

    pub fn battery_source_ref(&self) -> Option<Arc<BatterySource>> {
        self.battery_source.clone()
    }

    pub fn wifi_source_ref(&self) -> Option<Arc<WifiSource>> {
        self.wifi_source.clone()
    }

    pub fn bluetooth_source_ref(&self) -> Option<Arc<BluetoothSource>> {
        self.bluetooth_source.clone()
    }

    pub fn volume_source_ref(&self) -> Option<Arc<VolumeSource>> {
        self.volume_source.clone()
    }

    pub fn brightness_source_ref(&self) -> Option<Arc<BrightnessSource>> {
        self.brightness_source.clone()
    }

    async fn init_plugins(&mut self) {
        let loaded = self.plugin_manager.scan_plugins().await;
        tracing::info!(count = %loaded.len(), "plugin scan complete");
        let results = self.plugin_manager.start_all().await;
        for result in results {
            if let Err(e) = result {
                tracing::warn!(error = %e, "plugin start failed");
            }
        }
        self.start_plugin_ticker().await;
    }

    /// Drive periodic plugin `tick()` work on a fixed interval. All ready
    /// plugins are batched into one blocking hop per interval (see
    /// [`PluginManager::tick_all`]); crashes flow into the 6-2 auto-disable
    /// path. Stops when shutdown is signalled.
    async fn start_plugin_ticker(&mut self) {
        let manager = self.plugin_manager.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        manager.tick_all().await;
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("plugin ticker stopping");
                        break;
                    }
                }
            }
        });
        tracing::info!("plugin ticker started");
    }

    pub fn plugin_manager_ref(&self) -> Arc<PluginManager> {
        self.plugin_manager.clone()
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

        if let Some(conn) = self.dbus_system_conn.take() {
            drop(conn);
            tracing::info!("D-Bus system connection closed");
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
