use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::watch;

use dyd_types::{IpcCommand, IpcEnvelope, IpcEvent};
use crate::health::HealthCheck;

struct ClientGuard {
    counter: Arc<AtomicU32>,
}

impl ClientGuard {
    fn new(counter: Arc<AtomicU32>) -> Self {
        counter.fetch_add(1, Ordering::Relaxed);
        Self { counter }
    }
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

pub async fn handle_connection(
    stream: UnixStream,
    health: Arc<HealthCheck>,
    connected_clients: Arc<AtomicU32>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    let _guard = ClientGuard::new(connected_clients);

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                tracing::info!("shutting down IPC connection");
                break;
            }
            result = buf_reader.read_line(&mut line) => {
                match result {
                    Ok(0) => break,
                    Ok(_) => {
                        match IpcEnvelope::deserialize(line.trim().as_bytes()) {
                            Ok(envelope) => {
                                let response = handle_envelope(envelope, health.as_ref());
                                if let Some(resp) = response {
                                    if writer.write_all(&resp.serialize()).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to deserialize IPC frame");
                                let error_resp = IpcEnvelope::event(IpcEvent::ConnectionStatus(
                                    dyd_types::ConnectionStatusPayload { status: format!("parse_error: {e}") }
                                ));
                                let _ = writer.write_all(&error_resp.serialize()).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "IPC connection read error");
                        break;
                    }
                }
            }
        }
    }
}

fn handle_envelope(envelope: IpcEnvelope, health: &HealthCheck) -> Option<IpcEnvelope> {
    match envelope.payload {
        dyd_types::IpcPayload::Command(cmd) => handle_command(envelope.id, cmd, health),
        dyd_types::IpcPayload::Event(_) => None,
    }
}

fn handle_command(id: uuid::Uuid, cmd: IpcCommand, health: &HealthCheck) -> Option<IpcEnvelope> {
    let mut envelope = match cmd {
        IpcCommand::Ping => {
            let status = health.status();
            IpcEnvelope::event(IpcEvent::Pong(dyd_types::IpcPongPayload {
                status: status.status,
                uptime_secs: status.uptime_secs,
                version: status.version,
                connected_clients: status.connected_clients,
            }))
        }
        IpcCommand::GetState => {
            let status = health.status();
            IpcEnvelope::event(IpcEvent::ConnectionStatus(dyd_types::ConnectionStatusPayload {
                status: format!("ok:uptime={}", status.uptime_secs),
            }))
        }
        IpcCommand::CycleNext | IpcCommand::CyclePrev => {
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload { status: "ok:queued".to_string() }
            ))
        }
        IpcCommand::DismissActivity { activity_id: _ } => {
            IpcEnvelope::event(IpcEvent::QueueEmptied)
        }
        IpcCommand::MediaPlayPause { .. }
        | IpcCommand::MediaNext { .. }
        | IpcCommand::MediaPrevious { .. }
        | IpcCommand::MediaSeek { .. }
        | IpcCommand::MediaSetVolume { .. } => {
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload { status: "ok:media_command".to_string() }
            ))
        }
        IpcCommand::NotificationAction { .. } | IpcCommand::NotificationClose { .. } => {
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload { status: "ok:notification".to_string() }
            ))
        }
        IpcCommand::SetDnd { enabled } => {
            IpcEnvelope::event(IpcEvent::DndChanged(dyd_types::DndPayload { enabled }))
        }
        IpcCommand::SetVolume { .. } | IpcCommand::SetMute { .. } => {
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload { status: "ok:volume".to_string() }
            ))
        }
        IpcCommand::SetBrightness { .. } => {
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload { status: "ok:brightness".to_string() }
            ))
        }
        IpcCommand::SetBluetoothPowered { .. } => {
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload { status: "ok:bluetooth".to_string() }
            ))
        }
        IpcCommand::GetConfig => {
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload { status: "ok:config_not_implemented".to_string() }
            ))
        }
        IpcCommand::UpdateConfig { changes: _ } => {
            IpcEnvelope::event(IpcEvent::ConfigChanged(
                dyd_types::ConfigChangedPayload { changes: serde_json::json!({"status": "ok"}) }
            ))
        }
    };
    envelope.id = id;
    Some(envelope)
}
