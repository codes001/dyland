use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use tokio::net::UnixListener;
use tokio::sync::{broadcast, watch, oneshot};

use super::connection;
use crate::health::HealthCheck;
use crate::sources::media::MediaSource;
use crate::sources::notifications::NotificationSource;
use dyd_types::IpcEnvelope;

const SOCKET_PATH: &str = "/tmp/dyd-daemon.sock";

pub async fn start(
    health: Arc<HealthCheck>,
    connected_clients: Arc<AtomicU32>,
    mut shutdown_rx: watch::Receiver<()>,
    ready_tx: oneshot::Sender<()>,
    media_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    media_source: Option<Arc<MediaSource>>,
    notification_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    notification_source: Option<Arc<NotificationSource>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH)?;
    tracing::info!("IPC server listening on {}", SOCKET_PATH);
    let _ = ready_tx.send(());

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                tracing::info!("IPC server shutting down");
                break Ok(());
            }
            accept = listener.accept() => {
                match accept {
                    Ok((stream, _addr)) => {
                        tracing::debug!("New IPC connection");
                        let health = health.clone();
                        let clients = connected_clients.clone();
                        let rx = shutdown_rx.clone();
                        let media_rx = media_broadcast.as_ref().map(|tx| tx.subscribe());
                        let notification_rx = notification_broadcast.as_ref().map(|tx| tx.subscribe());
                        let source = media_source.clone();
                        let notif_source = notification_source.clone();
                        tokio::spawn(async move {
                            connection::handle_connection(stream, health, clients, rx, media_rx, notification_rx, source, notif_source).await;
                        });
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to accept IPC connection");
                    }
                }
            }
        }
    }
}

pub fn socket_path() -> &'static str {
    SOCKET_PATH
}
