# Epic 3 Review Diff

## Modified Files
diff --git a/dyd-daemon/src/ipc/connection.rs b/dyd-daemon/src/ipc/connection.rs
index 26a0fc5..0f15413 100644
--- a/dyd-daemon/src/ipc/connection.rs
+++ b/dyd-daemon/src/ipc/connection.rs
@@ -2,10 +2,11 @@ use std::sync::Arc;
 use std::sync::atomic::{AtomicU32, Ordering};
 use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
 use tokio::net::UnixStream;
-use tokio::sync::watch;
+use tokio::sync::{broadcast, mpsc, watch};
 
 use dyd_types::{IpcCommand, IpcEnvelope, IpcEvent};
 use crate::health::HealthCheck;
+use crate::sources::media::MediaSource;
 
 struct ClientGuard {
     counter: Arc<AtomicU32>,
@@ -29,12 +30,34 @@ pub async fn handle_connection(
     health: Arc<HealthCheck>,
     connected_clients: Arc<AtomicU32>,
     mut shutdown_rx: watch::Receiver<()>,
+    broadcast_rx: Option<broadcast::Receiver<IpcEnvelope>>,
+    media_source: Option<Arc<MediaSource>>,
 ) {
     let (reader, mut writer) = stream.into_split();
     let mut buf_reader = BufReader::new(reader);
     let mut line = String::new();
     let _guard = ClientGuard::new(connected_clients);
 
+    let (event_tx, mut event_rx) = mpsc::channel::<IpcEnvelope>(32);
+    if let Some(broadcast_rx) = broadcast_rx {
+        tokio::spawn(async move {
+            let mut rx = broadcast_rx;
+            loop {
+                match rx.recv().await {
+                    Ok(msg) => {
+                        if event_tx.send(msg).await.is_err() {
+                            break;
+                        }
+                    }
+                    Err(broadcast::error::RecvError::Closed) => break,
+                    Err(broadcast::error::RecvError::Lagged(n)) => {
+                        tracing::debug!(skipped = %n, "broadcast messages skipped");
+                    }
+                }
+            }
+        });
+    }
+
     loop {
         tokio::select! {
             _ = shutdown_rx.changed() => {
@@ -47,7 +70,7 @@ pub async fn handle_connection(
                     Ok(_) => {
                         match IpcEnvelope::deserialize(line.trim().as_bytes()) {
                             Ok(envelope) => {
-                                let response = handle_envelope(envelope, health.as_ref());
+                                let response = handle_envelope(envelope, health.as_ref(), &media_source).await;
                                 if let Some(resp) = response {
                                     if writer.write_all(&resp.serialize()).await.is_err() {
                                         break;
@@ -69,18 +92,23 @@ pub async fn handle_connection(
                     }
                 }
             }
+            Some(envelope) = event_rx.recv() => {
+                if writer.write_all(&envelope.serialize()).await.is_err() {
+                    break;
+                }
+            }
         }
     }
 }
 
-fn handle_envelope(envelope: IpcEnvelope, health: &HealthCheck) -> Option<IpcEnvelope> {
+async fn handle_envelope(envelope: IpcEnvelope, health: &HealthCheck, media_source: &Option<Arc<MediaSource>>) -> Option<IpcEnvelope> {
     match envelope.payload {
-        dyd_types::IpcPayload::Command(cmd) => handle_command(envelope.id, cmd, health),
+        dyd_types::IpcPayload::Command(cmd) => handle_command(envelope.id, cmd, health, media_source).await,
         dyd_types::IpcPayload::Event(_) => None,
     }
 }
 
-fn handle_command(id: uuid::Uuid, cmd: IpcCommand, health: &HealthCheck) -> Option<IpcEnvelope> {
+async fn handle_command(id: uuid::Uuid, cmd: IpcCommand, health: &HealthCheck, media_source: &Option<Arc<MediaSource>>) -> Option<IpcEnvelope> {
     let mut envelope = match cmd {
         IpcCommand::Ping => {
             let status = health.status();
@@ -105,15 +133,72 @@ fn handle_command(id: uuid::Uuid, cmd: IpcCommand, health: &HealthCheck) -> Opti
         IpcCommand::DismissActivity { activity_id: _ } => {
             IpcEnvelope::event(IpcEvent::QueueEmptied)
         }
-        IpcCommand::MediaPlayPause { .. }
-        | IpcCommand::MediaNext { .. }
-        | IpcCommand::MediaPrevious { .. }
-        | IpcCommand::MediaSeek { .. }
-        | IpcCommand::MediaSetVolume { .. } => {
+        IpcCommand::MediaPlayPause { ref player } => {
+            if let Some(source) = media_source.clone() {
+                let p = player.clone();
+                tokio::spawn(async move {
+                    let _ = source.send_command(&p, "PlayPause").await;
+                });
+            }
+            IpcEnvelope::event(IpcEvent::ConnectionStatus(
+                dyd_types::ConnectionStatusPayload { status: "ok:media_command".to_string() }
+            ))
+        }
+        IpcCommand::MediaNext { ref player } => {
+            if let Some(source) = media_source.clone() {
+                let p = player.clone();
+                tokio::spawn(async move {
+                    let _ = source.send_command(&p, "Next").await;
+                });
+            }
+            IpcEnvelope::event(IpcEvent::ConnectionStatus(
+                dyd_types::ConnectionStatusPayload { status: "ok:media_command".to_string() }
+            ))
+        }
+        IpcCommand::MediaPrevious { ref player } => {
+            if let Some(source) = media_source.clone() {
+                let p = player.clone();
+                tokio::spawn(async move {
+                    let _ = source.send_command(&p, "Previous").await;
+                });
+            }
             IpcEnvelope::event(IpcEvent::ConnectionStatus(
                 dyd_types::ConnectionStatusPayload { status: "ok:media_command".to_string() }
             ))
         }
+        IpcCommand::MediaSeek { ref player, position_us } => {
+            if let Some(source) = media_source.clone() {
+                let p = player.clone();
+                let pos = position_us as i64;
+                tokio::spawn(async move {
+                    let _ = source.seek(&p, pos).await;
+                });
+            }
+            IpcEnvelope::event(IpcEvent::ConnectionStatus(
+                dyd_types::ConnectionStatusPayload { status: "ok:media_command".to_string() }
+            ))
+        }
+        IpcCommand::MediaSetVolume { ref player, volume } => {
+            if let Some(source) = media_source.clone() {
+                let p = player.clone();
+                tokio::spawn(async move {
+                    let _ = source.set_volume(&p, volume).await;
+                });
+            }
+            IpcEnvelope::event(IpcEvent::ConnectionStatus(
+                dyd_types::ConnectionStatusPayload { status: "ok:media_command".to_string() }
+            ))
+        }
+        IpcCommand::CyclePlayer { .. } => {
+            if let Some(source) = media_source.clone() {
+                tokio::spawn(async move {
+                    let _ = source.cycle_player().await;
+                });
+            }
+            IpcEnvelope::event(IpcEvent::ConnectionStatus(
+                dyd_types::ConnectionStatusPayload { status: "ok:player_cycled".to_string() }
+            ))
+        }
         IpcCommand::NotificationAction { .. } | IpcCommand::NotificationClose { .. } => {
             IpcEnvelope::event(IpcEvent::ConnectionStatus(
                 dyd_types::ConnectionStatusPayload { status: "ok:notification".to_string() }
diff --git a/dyd-daemon/src/ipc/server.rs b/dyd-daemon/src/ipc/server.rs
index f9e06cb..82acda0 100644
--- a/dyd-daemon/src/ipc/server.rs
+++ b/dyd-daemon/src/ipc/server.rs
@@ -1,11 +1,12 @@
 use std::sync::Arc;
 use std::sync::atomic::AtomicU32;
 use tokio::net::UnixListener;
-use tokio::sync::watch;
-use tokio::sync::oneshot;
+use tokio::sync::{broadcast, watch, oneshot};
 
 use super::connection;
 use crate::health::HealthCheck;
+use crate::sources::media::MediaSource;
+use dyd_types::IpcEnvelope;
 
 const SOCKET_PATH: &str = "/tmp/dyd-daemon.sock";
 
@@ -14,6 +15,8 @@ pub async fn start(
     connected_clients: Arc<AtomicU32>,
     mut shutdown_rx: watch::Receiver<()>,
     ready_tx: oneshot::Sender<()>,
+    media_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
+    media_source: Option<Arc<MediaSource>>,
 ) -> Result<(), Box<dyn std::error::Error>> {
     let _ = std::fs::remove_file(SOCKET_PATH);
     let listener = UnixListener::bind(SOCKET_PATH)?;
@@ -33,8 +36,10 @@ pub async fn start(
                         let health = health.clone();
                         let clients = connected_clients.clone();
                         let rx = shutdown_rx.clone();
+                        let broadcast_rx = media_broadcast.as_ref().map(|tx| tx.subscribe());
+                        let source = media_source.clone();
                         tokio::spawn(async move {
-                            connection::handle_connection(stream, health, clients, rx).await;
+                            connection::handle_connection(stream, health, clients, rx, broadcast_rx, source).await;
                         });
                     }
                     Err(e) => {
diff --git a/dyd-daemon/src/lib.rs b/dyd-daemon/src/lib.rs
index 6eb24f9..c3c287f 100644
--- a/dyd-daemon/src/lib.rs
+++ b/dyd-daemon/src/lib.rs
@@ -9,6 +9,7 @@ pub use health::HealthCheck;
 pub use lifecycle::DaemonState;
 
 use sources::fullscreen::{poll_fullscreen, FullscreenState};
+use sources::media::MediaSource;
 
 use std::sync::Arc;
 use std::sync::atomic::AtomicU32;
@@ -18,8 +19,6 @@ use tokio::sync::oneshot;
 use tokio::task::JoinHandle;
 use tracing_subscriber::EnvFilter;
 
-
-
 static LOG_INIT: OnceLock<()> = OnceLock::new();
 
 pub struct Daemon {
@@ -35,6 +34,7 @@ pub struct Daemon {
     fullscreen_tx: watch::Sender<bool>,
     fullscreen_rx: watch::Receiver<bool>,
     activity_queue: Arc<tokio::sync::Mutex<queue::ActivityQueue>>,
+    media_source: Option<Arc<MediaSource>>,
 }
 
 impl Daemon {
@@ -59,6 +59,7 @@ impl Daemon {
             fullscreen_tx,
             fullscreen_rx,
             activity_queue,
+            media_source: None,
         }
     }
 
@@ -67,8 +68,10 @@ impl Daemon {
         tracing::info!(pid = std::process::id(), "dyd-daemon starting");
 
         self.connect_dbus().await;
+        self.init_media_source().await;
         self.start_ipc_server().await;
         self.start_fullscreen_poller().await;
+        self.start_media_poller().await;
         self.state = DaemonState::Running;
         self.health.set_ready();
         self.start_time = tokio::time::Instant::now();
@@ -120,14 +123,24 @@ impl Daemon {
         }
     }
 
+    async fn init_media_source(&mut self) {
+        let mut source = MediaSource::new();
+        if let Some(conn) = self.dbus_conn.as_ref() {
+            source.set_dbus_connection(conn.clone());
+        }
+        self.media_source = Some(Arc::new(source));
+    }
+
     async fn start_ipc_server(&mut self) {
         let health = self.health.clone();
         let clients = self.connected_clients.clone();
         let shutdown_rx = self.shutdown_rx.clone();
         let (ready_tx, ready_rx) = oneshot::channel::<()>();
+        let media_broadcast = self.media_source.as_ref().map(|m| m.event_broadcast());
+        let media_source = self.media_source.clone();
 
         let handle = tokio::spawn(async move {
-            if let Err(e) = ipc::server::start(health, clients, shutdown_rx, ready_tx).await {
+            if let Err(e) = ipc::server::start(health, clients, shutdown_rx, ready_tx, media_broadcast, media_source).await {
                 tracing::error!(error = %e, "IPC server failed");
             }
         });
@@ -155,6 +168,17 @@ impl Daemon {
         tracing::info!("fullscreen poller started");
     }
 
+    async fn start_media_poller(&mut self) {
+        if let Some(source) = self.media_source.as_ref() {
+            let shutdown_rx = self.shutdown_rx.clone();
+            let source = source.clone();
+            tokio::spawn(async move {
+                source.start_polling(shutdown_rx).await;
+            });
+            tracing::info!("media poller started");
+        }
+    }
+
     pub fn activity_queue(&self) -> Arc<tokio::sync::Mutex<queue::ActivityQueue>> {
         self.activity_queue.clone()
     }
@@ -167,6 +191,10 @@ impl Daemon {
         self.fullscreen_state.clone()
     }
 
+    pub fn media_source_ref(&self) -> Option<Arc<MediaSource>> {
+        self.media_source.clone()
+    }
+
     async fn shutdown(&mut self) {
         self.state = DaemonState::ShuttingDown;
         self.health.set_shutting_down();
diff --git a/dyd-daemon/src/sources/mod.rs b/dyd-daemon/src/sources/mod.rs
index 2928143..68e74f0 100644
--- a/dyd-daemon/src/sources/mod.rs
+++ b/dyd-daemon/src/sources/mod.rs
@@ -1 +1,2 @@
 pub mod fullscreen;
+pub mod media;
diff --git a/dyd-ui/src/components/ExpandedView.tsx b/dyd-ui/src/components/ExpandedView.tsx
index 40e6917..b2a1ac4 100644
--- a/dyd-ui/src/components/ExpandedView.tsx
+++ b/dyd-ui/src/components/ExpandedView.tsx
@@ -1,7 +1,10 @@
 import { useEffect, useRef } from "react";
 import { motion } from "motion/react";
 import { usePillStore } from "../stores/pill-store";
-import { SPRING, DURATION } from "../config/animation";
+import { useMediaStore } from "../stores/media-store";
+import MediaInfo from "./MediaInfo";
+import MediaControls from "./MediaControls";
+import { SPRING } from "../config/animation";
 
 export interface ExpandedViewProps {
   children: React.ReactNode;
@@ -15,11 +18,19 @@ function supportsBackdropBlur(): boolean {
 export default function ExpandedView({ children, onClose }: ExpandedViewProps) {
   const state = usePillStore((s) => s.state);
   const transitionTo = usePillStore((s) => s.transitionTo);
+  const media = useMediaStore((s) => s.media);
   const ref = useRef<HTMLDivElement>(null);
 
   const isVisible = state === "expanded";
   const hasBlur = supportsBackdropBlur();
 
+  useEffect(() => {
+    if (media?.playbackStatus === "playing" && state === "active") {
+      transitionTo("expanded");
+      onClose?.();
+    }
+  }, [media?.title, media?.playbackStatus, state, transitionTo, onClose]);
+
   useEffect(() => {
     if (!isVisible) return;
     const handleKeyDown = (e: KeyboardEvent) => {
@@ -68,6 +79,8 @@ export default function ExpandedView({ children, onClose }: ExpandedViewProps) {
       }}
     >
       {children}
+      <MediaInfo />
+      <MediaControls />
     </motion.div>
   );
 }
diff --git a/dyd-ui/src/index.css b/dyd-ui/src/index.css
index cdba80e..19a2c04 100644
--- a/dyd-ui/src/index.css
+++ b/dyd-ui/src/index.css
@@ -38,3 +38,18 @@ body,
   outline: 1px solid rgba(255, 255, 255, 0.5);
   outline-offset: 1px;
 }
+
+@keyframes marquee {
+  0% { transform: translateX(0); }
+  100% { transform: translateX(-50%); }
+}
+
+.marquee {
+  display: flex;
+  width: fit-content;
+  animation: marquee 10s linear infinite;
+}
+
+.marquee:hover {
+  animation-play-state: paused;
+}
diff --git a/dyd-ui/src/ipc/client.ts b/dyd-ui/src/ipc/client.ts
index fa3dcd4..d758cee 100644
--- a/dyd-ui/src/ipc/client.ts
+++ b/dyd-ui/src/ipc/client.ts
@@ -2,7 +2,8 @@ import { invoke } from "@tauri-apps/api/core";
 import { listen, type UnlistenFn } from "@tauri-apps/api/event";
 import { usePillStore } from "../stores/pill-store";
 import { useActivityStore } from "../stores/activity-store";
-import type { IpcEnvelope } from "@types/index";
+import { useMediaStore } from "../stores/media-store";
+import type { IpcEnvelope } from "../../../types/src/index";
 
 type ConnectionCallback = (status: ConnectionStatus) => void;
 
@@ -89,6 +90,33 @@ class IpcClient {
       case "ConfigChanged": {
         break;
       }
+      case "MediaChanged": {
+        const p = envelope.payload as {
+          player: string;
+          title: string;
+          artist: string;
+          album: string;
+          art_url: string | null;
+          length: number;
+          position: number;
+          playback_status: string;
+        };
+        if (p?.playback_status === "stopped" || (!p?.title && !p?.artist)) {
+          useMediaStore.getState().clearMedia();
+        } else if (p?.player) {
+          useMediaStore.getState().setMedia({
+            player: p.player,
+            title: p.title,
+            artist: p.artist,
+            album: p.album,
+            artUrl: p.art_url,
+            length: p.length,
+            position: p.position,
+            playbackStatus: p.playback_status,
+          });
+        }
+        break;
+      }
     }
   }
 
diff --git a/types/src/index.ts b/types/src/index.ts
index 8113b13..bc1e73b 100644
--- a/types/src/index.ts
+++ b/types/src/index.ts
@@ -23,6 +23,7 @@ export type IpcCommand =
   | { type: "MediaPrevious"; payload: { player: string } }
   | { type: "MediaSeek"; payload: { player: string; position_us: number } }
   | { type: "MediaSetVolume"; payload: { player: string; volume: number } }
+  | { type: "CyclePlayer"; payload: { player?: string } }
   | { type: "NotificationAction"; payload: { id: number; action_key: string; reply_text?: string } }
   | { type: "NotificationClose"; payload: { id: number } }
   | { type: "SetDnd"; payload: { enabled: boolean } }
diff --git a/types/src/lib.rs b/types/src/lib.rs
index dbd8cfb..a80a399 100644
--- a/types/src/lib.rs
+++ b/types/src/lib.rs
@@ -31,6 +31,7 @@ pub enum IpcCommand {
     MediaPrevious { player: String },
     MediaSeek { player: String, position_us: u64 },
     MediaSetVolume { player: String, volume: f64 },
+    CyclePlayer { player: Option<String> },
     NotificationAction { id: u32, action_key: String, reply_text: Option<String> },
     NotificationClose { id: u32 },
     SetDnd { enabled: bool },

## NEW FILE: dyd-daemon/src/sources/media.rs
diff --git a/dyd-daemon/src/sources/media.rs b/dyd-daemon/src/sources/media.rs
new file mode 100644
index 0000000..5b95852
--- /dev/null
+++ b/dyd-daemon/src/sources/media.rs
@@ -0,0 +1,441 @@
+use std::collections::HashMap;
+use std::sync::Arc;
+use tokio::sync::{broadcast, watch};
+use tokio::time::{interval, Duration};
+use zbus::Connection;
+
+use dyd_types::{IpcEnvelope, IpcEvent, MediaChangedPayload};
+
+#[derive(Debug, Clone)]
+struct PlayerState {
+    identity: String,
+    title: String,
+    artist: String,
+    album: String,
+    art_url: Option<String>,
+    length: u64,
+    playback_status: String,
+    position: u64,
+    last_active: tokio::time::Instant,
+}
+
+pub struct MediaSource {
+    dbus_conn: Option<Connection>,
+    players: Arc<tokio::sync::RwLock<HashMap<String, PlayerState>>>,
+    event_broadcast: broadcast::Sender<IpcEnvelope>,
+}
+
+impl MediaSource {
+    pub fn new() -> Self {
+        let (event_broadcast, _) = broadcast::channel(32);
+        Self {
+            dbus_conn: None,
+            players: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
+            event_broadcast,
+        }
+    }
+
+    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
+        self.event_broadcast.clone()
+    }
+}
+
+impl Default for MediaSource {
+    fn default() -> Self {
+        Self::new()
+    }
+}
+
+impl MediaSource {
+    pub fn set_dbus_connection(&mut self, conn: Connection) {
+        self.dbus_conn = Some(conn);
+    }
+
+    pub async fn start_polling(
+        self: Arc<Self>,
+        mut shutdown_rx: watch::Receiver<()>,
+    ) {
+        let mut tick = interval(Duration::from_secs(1));
+        tick.tick().await;
+
+        loop {
+            tokio::select! {
+                _ = shutdown_rx.changed() => {
+                    tracing::info!("media source shutting down");
+                    break;
+                }
+                _ = tick.tick() => {
+                    if let Err(e) = self.poll().await {
+                        tracing::debug!(error = %e, "media poll cycle error");
+                    }
+                }
+            }
+        }
+    }
+
+    async fn poll(&self) -> Result<(), Box<dyn std::error::Error>> {
+        let conn = match &self.dbus_conn {
+            Some(c) => c,
+            None => return Ok(()),
+        };
+
+        let body = conn.call_method(
+            Some("org.freedesktop.DBus"),
+            "/org/freedesktop/DBus",
+            Some("org.freedesktop.DBus"),
+            "ListNames",
+            &(),
+        ).await?.body();
+
+        let names: Vec<String> = body.deserialize()?;
+        let mpris_names: Vec<&str> = names.iter()
+            .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
+            .map(|s| s.as_str())
+            .collect();
+
+        if mpris_names.is_empty() {
+            let prev_active = self.players.read().await.len();
+            if prev_active > 0 {
+                self.players.write().await.clear();
+                self.emit_media_stopped().await;
+            }
+            return Ok(());
+        }
+
+        let mut new_players = HashMap::new();
+        let now = tokio::time::Instant::now();
+
+        for &name in &mpris_names {
+            let props = match self.fetch_player_properties(conn, name).await {
+                Ok(p) => p,
+                Err(e) => {
+                    tracing::debug!(player = %name, error = %e, "failed to fetch player properties");
+                    continue;
+                }
+            };
+
+            let was_active = self.players.read().await.contains_key(name);
+            let last_active = if props.playback_status == "playing" {
+                now
+            } else if was_active {
+                self.players.read().await.get(name)
+                    .map(|p| p.last_active)
+                    .unwrap_or(now)
+            } else {
+                now
+            };
+
+            new_players.insert(name.to_string(), PlayerState {
+                identity: name.to_string(),
+                title: props.title,
+                artist: props.artist,
+                album: props.album,
+                art_url: props.art_url,
+                length: props.length,
+                playback_status: props.playback_status,
+                position: props.position,
+                last_active,
+            });
+        }
+
+        let changed = {
+            let prev = self.players.read().await;
+            if prev.len() != new_players.len() {
+                true
+            } else {
+                prev.iter().zip(new_players.iter()).any(|((k1, v1), (k2, v2))| {
+                    k1 != k2
+                        || v1.playback_status != v2.playback_status
+                        || v1.title != v2.title
+                        || v1.artist != v2.artist
+                })
+            }
+        };
+
+        if changed {
+            *self.players.write().await = new_players;
+            let active = self.select_active_player().await;
+            match active {
+                Some(p) => {
+                    if p.playback_status != "stopped" {
+                        tracing::info!(
+                            player = %p.identity,
+                            title = %p.title,
+                            artist = %p.artist,
+                            status = %p.playback_status,
+                            "media changed"
+                        );
+                    }
+
+                    let payload = MediaChangedPayload {
+                        player: strip_bus_prefix(&p.identity),
+                        title: p.title.clone(),
+                        artist: p.artist.clone(),
+                        album: p.album.clone(),
+                        art_url: p.art_url.clone(),
+                        length: p.length,
+                        position: p.position,
+                        playback_status: p.playback_status.clone(),
+                    };
+
+                    if p.playback_status == "stopped" {
+                        self.emit_media_stopped().await;
+                    } else {
+                        self.emit_media_changed(payload).await;
+                    }
+                }
+                None => {
+                    self.emit_media_stopped().await;
+                }
+            }
+        }
+
+        Ok(())
+    }
+
+    async fn select_active_player(&self) -> Option<PlayerState> {
+        let players = self.players.read().await;
+        let playing = players.values()
+            .filter(|p| p.playback_status == "playing")
+            .max_by_key(|p| p.last_active);
+
+        if let Some(p) = playing {
+            return Some(p.clone());
+        }
+
+        players.values()
+            .filter(|p| p.playback_status == "paused")
+            .max_by_key(|p| p.last_active)
+            .cloned()
+    }
+
+    async fn fetch_player_properties(
+        &self,
+        conn: &Connection,
+        bus_name: &str,
+    ) -> Result<PlayerProps, Box<dyn std::error::Error>> {
+        let metadata_body = conn.call_method(
+            Some(bus_name),
+            "/org/mpris/MediaPlayer2",
+            Some("org.freedesktop.DBus.Properties"),
+            "Get",
+            &("org.mpris.MediaPlayer2.Player", "Metadata"),
+        ).await?.body();
+
+        let metadata: zvariant::Value = metadata_body.deserialize()?;
+
+        let status_body = conn.call_method(
+            Some(bus_name),
+            "/org/mpris/MediaPlayer2",
+            Some("org.freedesktop.DBus.Properties"),
+            "Get",
+            &("org.mpris.MediaPlayer2.Player", "PlaybackStatus"),
+        ).await?.body();
+
+        let playback_status: String = status_body.deserialize()?;
+
+        let position_body = conn.call_method(
+            Some(bus_name),
+            "/org/mpris/MediaPlayer2",
+            Some("org.freedesktop.DBus.Properties"),
+            "Get",
+            &("org.mpris.MediaPlayer2.Player", "Position"),
+        ).await?.body();
+
+        let position: u64 = position_body.deserialize()?;
+
+        let dict = match metadata.downcast_ref::<zvariant::Dict>() {
+            Ok(d) => d,
+            Err(_) => return Ok(PlayerProps::default()),
+        };
+
+        let title = get_dict_string(&dict, "xesam:title").unwrap_or_default();
+        let artist = get_dict_string(&dict, "xesam:artist").unwrap_or_default();
+        let album = get_dict_string(&dict, "xesam:album").unwrap_or_default();
+        let art_url = get_dict_string(&dict, "mpris:artUrl");
+        let length: u64 = get_dict_u64(&dict, "mpris:length").unwrap_or(0) / 1000;
+
+        Ok(PlayerProps {
+            title,
+            artist,
+            album,
+            art_url,
+            length,
+            playback_status: playback_status.to_lowercase(),
+            position,
+        })
+    }
+
+    pub async fn send_command(&self, player: &str, method: &str) -> Result<(), Box<dyn std::error::Error>> {
+        let conn = match &self.dbus_conn {
+            Some(c) => c,
+            None => return Err("no D-Bus connection".into()),
+        };
+
+        let bus_name = format!("org.mpris.MediaPlayer2.{}", player);
+        conn.call_method(
+            Some(bus_name.as_str()),
+            "/org/mpris/MediaPlayer2",
+            Some("org.mpris.MediaPlayer2.Player"),
+            method,
+            &(),
+        ).await?;
+
+        tracing::debug!(player = %player, method = %method, "media command sent");
+        Ok(())
+    }
+
+    pub async fn seek(&self, player: &str, offset_us: i64) -> Result<(), Box<dyn std::error::Error>> {
+        let conn = match &self.dbus_conn {
+            Some(c) => c,
+            None => return Err("no D-Bus connection".into()),
+        };
+
+        let bus_name = format!("org.mpris.MediaPlayer2.{}", player);
+        conn.call_method(
+            Some(bus_name.as_str()),
+            "/org/mpris/MediaPlayer2",
+            Some("org.mpris.MediaPlayer2.Player"),
+            "Seek",
+            &(offset_us),
+        ).await?;
+
+        tracing::debug!(player = %player, offset_us = %offset_us, "media seek sent");
+        Ok(())
+    }
+
+    pub async fn set_volume(&self, player: &str, volume: f64) -> Result<(), Box<dyn std::error::Error>> {
+        let conn = match &self.dbus_conn {
+            Some(c) => c,
+            None => return Err("no D-Bus connection".into()),
+        };
+
+        let bus_name = format!("org.mpris.MediaPlayer2.{}", player);
+        conn.call_method(
+            Some(bus_name.as_str()),
+            "/org/mpris/MediaPlayer2",
+            Some("org.freedesktop.DBus.Properties"),
+            "Set",
+            &("org.mpris.MediaPlayer2.Player", "Volume", &volume),
+        ).await?;
+
+        tracing::debug!(player = %player, volume = %volume, "media volume set");
+        Ok(())
+    }
+
+    pub async fn list_players(&self) -> Vec<String> {
+        self.players.read().await.keys()
+            .map(|k| strip_bus_prefix(k))
+            .collect()
+    }
+
+    pub async fn cycle_player(&self) -> Option<String> {
+        let players = self.players.read().await;
+        if players.is_empty() {
+            return None;
+        }
+        let active = self.select_active_player().await;
+        let names: Vec<String> = players.keys().cloned().collect();
+        let current_idx = active.as_ref()
+            .and_then(|a| names.iter().position(|n| n == &a.identity))
+            .unwrap_or(0);
+        let next_idx = (current_idx + 1) % names.len();
+        let next_name = names[next_idx].clone();
+
+        let next_state = players.get(&next_name).cloned();
+        drop(players);
+
+        if let Some(p) = next_state {
+            let payload = MediaChangedPayload {
+                player: strip_bus_prefix(&p.identity),
+                title: p.title.clone(),
+                artist: p.artist.clone(),
+                album: p.album.clone(),
+                art_url: p.art_url.clone(),
+                length: p.length,
+                position: p.position,
+                playback_status: p.playback_status.clone(),
+            };
+            self.emit_media_changed(payload).await;
+            Some(strip_bus_prefix(&next_name))
+        } else {
+            None
+        }
+    }
+
+    async fn emit_media_changed(&self, payload: MediaChangedPayload) {
+        let envelope = IpcEnvelope::event(IpcEvent::MediaChanged(payload));
+        let _ = self.event_broadcast.send(envelope);
+    }
+
+    async fn emit_media_stopped(&self) {
+        let envelope = IpcEnvelope::event(IpcEvent::MediaChanged(MediaChangedPayload {
+            player: String::new(),
+            title: String::new(),
+            artist: String::new(),
+            album: String::new(),
+            art_url: None,
+            length: 0,
+            position: 0,
+            playback_status: "stopped".to_string(),
+        }));
+        let _ = self.event_broadcast.send(envelope);
+    }
+}
+
+#[derive(Default)]
+struct PlayerProps {
+    title: String,
+    artist: String,
+    album: String,
+    art_url: Option<String>,
+    length: u64,
+    playback_status: String,
+    position: u64,
+}
+
+fn get_dict_string(dict: &zvariant::Dict, key: &str) -> Option<String> {
+    for pair in dict.iter() {
+        if let Ok(k) = pair.0.downcast_ref::<String>() {
+            if k == key {
+                if let Ok(v) = pair.1.downcast_ref::<String>() {
+                    return Some(v.clone());
+                }
+                if let Ok(arr) = pair.1.downcast_ref::<zvariant::Array>() {
+                    for item in arr.iter() {
+                        if let Ok(s) = item.downcast_ref::<String>() {
+                            return Some(s.clone());
+                        }
+                    }
+                }
+            }
+        }
+    }
+    None
+}
+
+fn get_dict_u64(dict: &zvariant::Dict, key: &str) -> Option<u64> {
+    for pair in dict.iter() {
+        if let Ok(k) = pair.0.downcast_ref::<String>() {
+            if k == key {
+                if let Ok(v) = pair.1.downcast_ref::<u64>() {
+                    return Some(v);
+                }
+                if let Ok(v) = pair.1.downcast_ref::<i64>() {
+                    return Some(v as u64);
+                }
+                if let Ok(v) = pair.1.downcast_ref::<u32>() {
+                    return Some(v as u64);
+                }
+                if let Ok(v) = pair.1.downcast_ref::<i32>() {
+                    return Some(v as u64);
+                }
+            }
+        }
+    }
+    None
+}
+
+fn strip_bus_prefix(name: &str) -> String {
+    name.strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(name).to_string()
+}
