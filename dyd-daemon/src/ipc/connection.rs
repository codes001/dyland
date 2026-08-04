use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, mpsc, watch};

use super::context::SourceContext;
use crate::health::HealthCheck;
use dyd_types::{IpcCommand, IpcEnvelope, IpcEvent};

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

fn spawn_broadcast_forwarder(
    mut rx: broadcast::Receiver<IpcEnvelope>,
    event_tx: mpsc::Sender<IpcEnvelope>,
) {
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if event_tx.try_send(msg).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = %n, "broadcast messages skipped");
                }
            }
        }
    });
}

pub async fn handle_connection(
    stream: UnixStream,
    health: Arc<HealthCheck>,
    connected_clients: Arc<AtomicU32>,
    mut shutdown_rx: watch::Receiver<()>,
    ctx: SourceContext,
) {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    let _guard = ClientGuard::new(connected_clients);

    let (event_tx, mut event_rx) = mpsc::channel::<IpcEnvelope>(64);
    for broadcast_rx in ctx.broadcast_receivers() {
        spawn_broadcast_forwarder(broadcast_rx, event_tx.clone());
    }

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
                                let response = handle_envelope(envelope, health.as_ref(), &ctx).await;
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
            Some(envelope) = event_rx.recv() => {
                if writer.write_all(&envelope.serialize()).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn handle_envelope(
    envelope: IpcEnvelope,
    health: &HealthCheck,
    ctx: &SourceContext,
) -> Option<IpcEnvelope> {
    match envelope.payload {
        dyd_types::IpcPayload::Command(cmd) => handle_command(envelope.id, cmd, health, ctx).await,
        dyd_types::IpcPayload::Event(_) => None,
    }
}

async fn handle_command(
    id: uuid::Uuid,
    cmd: IpcCommand,
    health: &HealthCheck,
    ctx: &SourceContext,
) -> Option<IpcEnvelope> {
    let media_source = &ctx.media_source;
    let notification_source = &ctx.notification_source;
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
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload {
                    status: format!("ok:uptime={}", status.uptime_secs),
                },
            ))
        }
        IpcCommand::CycleNext | IpcCommand::CyclePrev => IpcEnvelope::event(
            IpcEvent::ConnectionStatus(dyd_types::ConnectionStatusPayload {
                status: "ok:queued".to_string(),
            }),
        ),
        IpcCommand::DismissActivity { activity_id: _ } => {
            IpcEnvelope::event(IpcEvent::QueueEmptied)
        }
        IpcCommand::MediaPlayPause { ref player } => {
            if let Some(source) = media_source.clone() {
                let p = player.clone();
                tokio::spawn(async move {
                    if let Err(e) = source.send_command(&p, "PlayPause").await {
                        tracing::warn!(error = %e, player = %p, command = "PlayPause", "media command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:media_command".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_media_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::MediaNext { ref player } => {
            if let Some(source) = media_source.clone() {
                let p = player.clone();
                tokio::spawn(async move {
                    if let Err(e) = source.send_command(&p, "Next").await {
                        tracing::warn!(error = %e, player = %p, command = "Next", "media command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:media_command".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_media_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::MediaPrevious { ref player } => {
            if let Some(source) = media_source.clone() {
                let p = player.clone();
                tokio::spawn(async move {
                    if let Err(e) = source.send_command(&p, "Previous").await {
                        tracing::warn!(error = %e, player = %p, command = "Previous", "media command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:media_command".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_media_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::MediaSeek {
            ref player,
            position_us,
        } => {
            if let Some(source) = media_source.clone() {
                let p = player.clone();
                tokio::spawn(async move {
                    if let Err(e) = source.seek(&p, position_us).await {
                        tracing::warn!(error = %e, player = %p, command = "Seek", "media command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:media_command".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_media_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::MediaSetVolume { ref player, volume } => {
            let volume = if volume.is_finite() {
                volume.clamp(0.0, 1.0)
            } else {
                0.0
            };
            if let Some(source) = media_source.clone() {
                let p = player.clone();
                tokio::spawn(async move {
                    if let Err(e) = source.set_volume(&p, volume).await {
                        tracing::warn!(error = %e, player = %p, command = "SetVolume", "media command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:media_command".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_media_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::CyclePlayer { .. } => {
            if let Some(source) = media_source.clone() {
                tokio::spawn(async move {
                    source.cycle_player().await;
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:player_cycled".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_media_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::NotificationAction {
            id,
            ref action_key,
            reply_text: _,
        } => {
            let action_key = action_key.clone();
            if let Some(source) = notification_source.clone() {
                tokio::spawn(async move {
                    if let Err(e) = source.invoke_action(id, &action_key).await {
                        tracing::warn!(error = %e, id, action_key = %action_key, "notification action failed");
                    }
                });
            } else {
                tracing::debug!(id, action_key = %action_key, "no notification source for action");
            }
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload {
                    status: "ok:notification_action".to_string(),
                },
            ))
        }
        IpcCommand::NotificationClose { id } => {
            if let Some(source) = notification_source.clone() {
                tokio::spawn(async move {
                    if let Err(e) = source.close_notification(id).await {
                        tracing::warn!(error = %e, id, "notification close failed");
                    }
                });
            } else {
                tracing::debug!(id, "no notification source for close");
            }
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload {
                    status: "ok:notification_close".to_string(),
                },
            ))
        }
        IpcCommand::SetDnd { enabled } => {
            IpcEnvelope::event(IpcEvent::DndChanged(dyd_types::DndPayload { enabled }))
        }
        IpcCommand::SetVolume { volume } => {
            let volume = if volume.is_finite() {
                volume.clamp(0.0, 1.0)
            } else {
                0.0
            };
            if let Some(source) = ctx.volume_source.clone() {
                tokio::spawn(async move {
                    if let Err(e) = source.set_volume(volume).await {
                        tracing::warn!(error = %e, volume = %volume, command = "SetVolume", "volume command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:volume".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_volume_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::SetMute { muted } => {
            if let Some(source) = ctx.volume_source.clone() {
                tokio::spawn(async move {
                    if let Err(e) = source.set_mute(muted).await {
                        tracing::warn!(error = %e, muted = %muted, command = "SetMute", "mute command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:volume".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_volume_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::SetBrightness { brightness } => {
            let brightness = if brightness.is_finite() {
                brightness.clamp(0.0, 100.0)
            } else {
                0.0
            } as u8;
            if let Some(source) = ctx.brightness_source.clone() {
                tokio::spawn(async move {
                    if let Err(e) = source.set_brightness(brightness).await {
                        tracing::warn!(error = %e, brightness = %brightness, command = "SetBrightness", "brightness command failed");
                    }
                });
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "ok:brightness".to_string(),
                    },
                ))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_brightness_source".to_string(),
                    },
                ))
            }
        }
        IpcCommand::SetBluetoothPowered { powered } => {
            if let Some(source) = ctx.bluetooth_source.clone() {
                tokio::spawn(async move {
                    if let Err(e) = source.set_powered(powered).await {
                        tracing::warn!(error = %e, powered = %powered, "bluetooth powered set failed");
                    }
                });
            } else {
                tracing::debug!("no bluetooth source for set powered");
                return Some(IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_bluetooth_source".to_string(),
                    },
                )));
            }
            IpcEnvelope::event(IpcEvent::ConnectionStatus(
                dyd_types::ConnectionStatusPayload {
                    status: "ok:bluetooth".to_string(),
                },
            ))
        }
        IpcCommand::GetConfig => IpcEnvelope::event(IpcEvent::ConnectionStatus(
            dyd_types::ConnectionStatusPayload {
                status: "ok:config_not_implemented".to_string(),
            },
        )),
        IpcCommand::UpdateConfig { changes: _ } => {
            IpcEnvelope::event(IpcEvent::ConfigChanged(dyd_types::ConfigChangedPayload {
                changes: serde_json::json!({"status": "ok"}),
            }))
        }
        IpcCommand::GetPlugins => {
            if let Some(manager) = ctx.plugin_manager.clone() {
                let plugins = manager.list().await;
                let payloads: Vec<dyd_types::PluginHealthPayload> = plugins
                    .iter()
                    .map(crate::plugins::health_payload)
                    .collect();
                IpcEnvelope::event(IpcEvent::PluginList(dyd_types::PluginListPayload {
                    plugins: payloads,
                }))
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_plugin_manager".to_string(),
                    },
                ))
            }
        }
        IpcCommand::GetPluginHealth { ref name } => {
            if let Some(manager) = ctx.plugin_manager.clone() {
                let name = name.clone();
                match manager.get(&name).await {
                    Some(h) => {
                        let payload = crate::plugins::health_payload(&h);
                        IpcEnvelope::event(IpcEvent::PluginChanged(payload))
                    }
                    None => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("error:plugin_not_found:{name}"),
                        },
                    )),
                }
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_plugin_manager".to_string(),
                    },
                ))
            }
        }
        IpcCommand::InstallPlugin { ref path } => {
            if let Some(manager) = ctx.plugin_manager.clone() {
                let path = path.clone();
                match manager.install_plugin(std::path::Path::new(&path)).await {
                    Ok(name) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("ok:plugin_installed:{name}"),
                        },
                    )),
                    Err(e) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("error:plugin_install:{e}"),
                        },
                    )),
                }
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_plugin_manager".to_string(),
                    },
                ))
            }
        }
        IpcCommand::SetPluginEnabled { ref name, enabled } => {
            if let Some(manager) = ctx.plugin_manager.clone() {
                let name = name.clone();
                match manager.set_enabled(&name, enabled).await {
                    Ok(()) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("ok:plugin_enabled:{name}"),
                        },
                    )),
                    Err(e) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("error:plugin_enabled:{e}"),
                        },
                    )),
                }
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_plugin_manager".to_string(),
                    },
                ))
            }
        }
        IpcCommand::UninstallPlugin { ref name } => {
            if let Some(manager) = ctx.plugin_manager.clone() {
                let name = name.clone();
                match manager.uninstall_plugin(&name).await {
                    Ok(()) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("ok:plugin_uninstalled:{name}"),
                        },
                    )),
                    Err(e) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("error:plugin_uninstall:{e}"),
                        },
                    )),
                }
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_plugin_manager".to_string(),
                    },
                ))
            }
        }
        IpcCommand::SetPluginConfig { ref name, config } => {
            if let Some(manager) = ctx.plugin_manager.clone() {
                let name = name.clone();
                match manager.set_config(&name, config).await {
                    Ok(()) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("ok:plugin_config:{name}"),
                        },
                    )),
                    Err(e) => IpcEnvelope::event(IpcEvent::ConnectionStatus(
                        dyd_types::ConnectionStatusPayload {
                            status: format!("error:plugin_config:{e}"),
                        },
                    )),
                }
            } else {
                IpcEnvelope::event(IpcEvent::ConnectionStatus(
                    dyd_types::ConnectionStatusPayload {
                        status: "error:no_plugin_manager".to_string(),
                    },
                ))
            }
        }
    };
    envelope.id = id;
    Some(envelope)
}
