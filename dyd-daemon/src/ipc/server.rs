use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::{oneshot, watch};

use super::connection;
use super::context::SourceContext;
use crate::health::HealthCheck;

const SOCKET_PATH: &str = "/tmp/dyd-daemon.sock";

pub async fn start(
    health: Arc<HealthCheck>,
    connected_clients: Arc<AtomicU32>,
    mut shutdown_rx: watch::Receiver<()>,
    ready_tx: oneshot::Sender<()>,
    sources: SourceContext,
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
                        let ctx = sources.clone();
                        tokio::spawn(async move {
                            connection::handle_connection(stream, health, clients, rx, ctx).await;
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
