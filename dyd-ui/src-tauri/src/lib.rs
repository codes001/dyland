use std::sync::Mutex;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::watch;

use dyd_types::{IpcCommand, IpcEnvelope};
use tauri::{AppHandle, Emitter, Manager};

struct DaemonClient {
    writer: Option<tokio::sync::mpsc::Sender<IpcEnvelope>>,
    shutdown_tx: watch::Sender<bool>,
}

impl DaemonClient {
    fn new() -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            writer: None,
            shutdown_tx,
        }
    }
}

#[tauri::command]
async fn connect_daemon(
    app: AppHandle,
    state: tauri::State<'_, Mutex<DaemonClient>>,
) -> Result<String, String> {
    let (writer_tx, mut writer_rx) = tokio::sync::mpsc::channel::<IpcEnvelope>(64);
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    {
        let mut client = state.lock().map_err(|e| e.to_string())?;
        let _ = client.shutdown_tx.send(true);
        client.writer = Some(writer_tx.clone());
        client.shutdown_tx = shutdown_tx;
    }

    let app_handle = app.clone();

    tokio::spawn(async move {
        let stream = match UnixStream::connect("/tmp/dyd-daemon.sock").await {
            Ok(s) => {
                let _ = app_handle.emit("daemon-status", serde_json::json!({"status": "connected"}));
                s
            }
            Err(e) => {
                let _ = app_handle.emit("daemon-status", serde_json::json!({"status": "disconnected", "error": e.to_string()}));
                return;
            }
        };

        let (reader, mut writer_half) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    break;
                }
                Some(envelope) = writer_rx.recv() => {
                    let data = envelope.serialize();
                    if writer_half.write_all(&data).await.is_err() {
                        break;
                    }
                }
                result = buf_reader.read_line(&mut line) => {
                    match result {
                        Ok(0) => break,
                        Ok(_) => {
                            if let Ok(envelope) = IpcEnvelope::deserialize(line.trim().as_bytes()) {
                                let _ = app_handle.emit("daemon-message", &envelope);
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        let _ = app_handle.emit("daemon-status", serde_json::json!({"status": "disconnected"}));
    });

    Ok("connecting".to_string())
}

#[tauri::command]
async fn send_command(
    command: String,
    state: tauri::State<'_, Mutex<DaemonClient>>,
) -> Result<(), String> {
    let tx = {
        let client = state.lock().map_err(|e| e.to_string())?;
        client.writer.clone()
    };
    let Some(tx) = tx else {
        return Err("not connected".to_string());
    };
    let cmd: IpcCommand = serde_json::from_str(&command).map_err(|e| e.to_string())?;
    let envelope = IpcEnvelope::command(cmd);
    tx.send(envelope).await.map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(DaemonClient::new()))
        .setup(|app| {
            let window = match app.get_webview_window("main") {
                Some(w) => w,
                None => return Ok(()),
            };
            if let Ok(Some(monitor)) = window.primary_monitor() {
                let monitor_size = monitor.size();
                let _ = window.set_size(tauri::PhysicalSize::new(monitor_size.width, 100));
                let _ = window.set_position(tauri::PhysicalPosition::new(0, 0));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![connect_daemon, send_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
