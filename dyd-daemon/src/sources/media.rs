use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, Duration};
use zbus::Connection;

use dyd_types::{IpcEnvelope, IpcEvent, MediaChangedPayload};

#[derive(Debug, Clone)]
struct PlayerState {
    identity: String,
    title: String,
    artist: String,
    album: String,
    art_url: Option<String>,
    length: u64,
    playback_status: String,
    position: u64,
    last_active: tokio::time::Instant,
}

pub struct MediaSource {
    dbus_conn: Option<Connection>,
    players: Arc<tokio::sync::RwLock<HashMap<String, PlayerState>>>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
}

impl MediaSource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            dbus_conn: None,
            players: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            event_broadcast,
        }
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.event_broadcast.clone()
    }
}

impl Default for MediaSource {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaSource {
    pub fn set_dbus_connection(&mut self, conn: Connection) {
        self.dbus_conn = Some(conn);
    }

    pub async fn start_polling(
        self: Arc<Self>,
        mut shutdown_rx: watch::Receiver<()>,
    ) {
        let mut tick = interval(Duration::from_secs(1));
        tick.tick().await;

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    tracing::info!("media source shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = self.poll().await {
                        tracing::debug!(error = %e, "media poll cycle error");
                    }
                }
            }
        }
    }

    async fn poll(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Ok(()),
        };

        let body = conn.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        ).await?.body();

        let names: Vec<String> = body.deserialize()?;
        let mpris_names: Vec<&str> = names.iter()
            .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
            .map(|s| s.as_str())
            .collect();

        if mpris_names.is_empty() {
            let prev_active = self.players.read().await.len();
            if prev_active > 0 {
                self.players.write().await.clear();
                self.emit_media_stopped().await;
            }
            return Ok(());
        }

        let mut new_players = HashMap::new();
        let now = tokio::time::Instant::now();

        for &name in &mpris_names {
            let is_new = !self.players.read().await.contains_key(name);

            let props = match self.fetch_player_properties(conn, name).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::debug!(player = %name, error = %e, "failed to fetch player properties");
                    continue;
                }
            };

            if is_new {
                tracing::info!(player = %name, status = %props.playback_status, "media player discovered");
            }

            let was_active = self.players.read().await.contains_key(name);
            let last_active = if props.playback_status == "playing" {
                now
            } else if was_active {
                self.players.read().await.get(name)
                    .map(|p| p.last_active)
                    .unwrap_or(now)
            } else {
                now
            };

            new_players.insert(name.to_string(), PlayerState {
                identity: name.to_string(),
                title: props.title,
                artist: props.artist,
                album: props.album,
                art_url: props.art_url,
                length: props.length,
                playback_status: props.playback_status,
                position: props.position,
                last_active,
            });
        }

        let changed = {
            let prev = self.players.read().await;
            if prev.len() != new_players.len() {
                true
            } else {
                let mut prev_sorted: Vec<(&String, &PlayerState)> = prev.iter().collect();
                prev_sorted.sort_by(|a, b| a.0.cmp(b.0));
                let mut new_sorted: Vec<(&String, &PlayerState)> = new_players.iter().collect();
                new_sorted.sort_by(|a, b| a.0.cmp(b.0));
                prev_sorted.iter().zip(new_sorted.iter()).any(|((k1, v1), (k2, v2))| {
                    k1 != k2
                        || v1.playback_status != v2.playback_status
                        || v1.title != v2.title
                        || v1.artist != v2.artist
                        || v1.album != v2.album
                        || v1.art_url != v2.art_url
                        || v1.length != v2.length
                        || v1.position != v2.position
                })
            }
        };

        if changed {
            *self.players.write().await = new_players;
            let active = self.select_active_player().await;
            match active {
                Some(p) => {
                    if p.playback_status != "stopped" {
                        tracing::debug!(
                            player = %p.identity,
                            title = %p.title,
                            artist = %p.artist,
                            status = %p.playback_status,
                            "media changed"
                        );
                    }

                    let payload = MediaChangedPayload {
                        player: strip_bus_prefix(&p.identity),
                        title: p.title.clone(),
                        artist: p.artist.clone(),
                        album: p.album.clone(),
                        art_url: p.art_url.clone(),
                        length: p.length,
                        position: p.position,
                        playback_status: p.playback_status.clone(),
                    };

                    if p.playback_status == "stopped" {
                        self.emit_media_stopped().await;
                    } else {
                        self.emit_media_changed(payload).await;
                    }
                }
                None => {
                    self.emit_media_stopped().await;
                }
            }
        }

        Ok(())
    }

    async fn select_active_player(&self) -> Option<PlayerState> {
        let players = self.players.read().await;
        let playing = players.values()
            .filter(|p| p.playback_status == "playing")
            .max_by_key(|p| p.last_active);

        if let Some(p) = playing {
            return Some(p.clone());
        }

        let paused = players.values()
            .filter(|p| p.playback_status == "paused")
            .max_by_key(|p| p.last_active);

        if let Some(p) = paused {
            return Some(p.clone());
        }

        players.values()
            .max_by_key(|p| p.last_active)
            .cloned()
    }

    async fn fetch_player_properties(
        &self,
        conn: &Connection,
        bus_name: &str,
    ) -> Result<PlayerProps, Box<dyn std::error::Error>> {
        let metadata_body = conn.call_method(
            Some(bus_name),
            "/org/mpris/MediaPlayer2",
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.mpris.MediaPlayer2.Player", "Metadata"),
        ).await?.body();

        let metadata: zvariant::Value = metadata_body.deserialize()?;

        let status_body = conn.call_method(
            Some(bus_name),
            "/org/mpris/MediaPlayer2",
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.mpris.MediaPlayer2.Player", "PlaybackStatus"),
        ).await?.body();

        let playback_status: String = status_body.deserialize()?;

        let position_body = conn.call_method(
            Some(bus_name),
            "/org/mpris/MediaPlayer2",
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.mpris.MediaPlayer2.Player", "Position"),
        ).await?.body();

        let position_us: u64 = position_body.deserialize()?;

        let dict = match metadata.downcast_ref::<zvariant::Dict>() {
            Ok(d) => d,
            Err(_) => return Ok(PlayerProps::default()),
        };

        let title = get_dict_string(&dict, "xesam:title").unwrap_or_default();
        let artist = get_dict_string(&dict, "xesam:artist").unwrap_or_default();
        let album = get_dict_string(&dict, "xesam:album").unwrap_or_default();
        let art_url = get_dict_string(&dict, "mpris:artUrl");
        let length: u64 = get_dict_u64(&dict, "mpris:length").unwrap_or(0) / 1000;
        let position: u64 = position_us / 1000;

        Ok(PlayerProps {
            title,
            artist,
            album,
            art_url,
            length,
            playback_status: playback_status.to_lowercase(),
            position,
        })
    }

    pub async fn send_command(&self, player: &str, method: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Err("no D-Bus connection".into()),
        };

        let bus_name = format!("org.mpris.MediaPlayer2.{}", player);
        conn.call_method(
            Some(bus_name.as_str()),
            "/org/mpris/MediaPlayer2",
            Some("org.mpris.MediaPlayer2.Player"),
            method,
            &(),
        ).await?;

        tracing::debug!(player = %player, method = %method, "media command sent");
        Ok(())
    }

    pub async fn seek(&self, player: &str, offset_us: i64) -> Result<(), Box<dyn std::error::Error>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Err("no D-Bus connection".into()),
        };

        let bus_name = format!("org.mpris.MediaPlayer2.{}", player);
        conn.call_method(
            Some(bus_name.as_str()),
            "/org/mpris/MediaPlayer2",
            Some("org.mpris.MediaPlayer2.Player"),
            "Seek",
            &(offset_us),
        ).await?;

        tracing::debug!(player = %player, offset_us = %offset_us, "media seek sent");
        Ok(())
    }

    pub async fn set_volume(&self, player: &str, volume: f64) -> Result<(), Box<dyn std::error::Error>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Err("no D-Bus connection".into()),
        };

        let bus_name = format!("org.mpris.MediaPlayer2.{}", player);
        conn.call_method(
            Some(bus_name.as_str()),
            "/org/mpris/MediaPlayer2",
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &("org.mpris.MediaPlayer2.Player", "Volume", &volume),
        ).await?;

        tracing::debug!(player = %player, volume = %volume, "media volume set");
        Ok(())
    }

    pub async fn list_players(&self) -> Vec<String> {
        self.players.read().await.keys()
            .map(|k| strip_bus_prefix(k))
            .collect()
    }

    pub async fn cycle_player(&self) -> Option<String> {
        let players = self.players.read().await;
        if players.is_empty() {
            return None;
        }
        let active = self.select_active_player().await;
        let names: Vec<String> = players.keys().cloned().collect();
        let current_idx = active.as_ref()
            .and_then(|a| names.iter().position(|n| n == &a.identity))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % names.len();
        let next_name = names[next_idx].clone();

        let next_state = players.get(&next_name).cloned();
        drop(players);

        if let Some(p) = next_state {
            let payload = MediaChangedPayload {
                player: strip_bus_prefix(&p.identity),
                title: p.title.clone(),
                artist: p.artist.clone(),
                album: p.album.clone(),
                art_url: p.art_url.clone(),
                length: p.length,
                position: p.position,
                playback_status: p.playback_status.clone(),
            };
            self.emit_media_changed(payload).await;
            Some(strip_bus_prefix(&next_name))
        } else {
            None
        }
    }

    async fn emit_media_changed(&self, payload: MediaChangedPayload) {
        let envelope = IpcEnvelope::event(IpcEvent::MediaChanged(payload));
        let _ = self.event_broadcast.send(envelope);
    }

    async fn emit_media_stopped(&self) {
        let envelope = IpcEnvelope::event(IpcEvent::MediaChanged(MediaChangedPayload {
            player: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            art_url: None,
            length: 0,
            position: 0,
            playback_status: "stopped".to_string(),
        }));
        let _ = self.event_broadcast.send(envelope);
    }
}

#[derive(Default)]
struct PlayerProps {
    title: String,
    artist: String,
    album: String,
    art_url: Option<String>,
    length: u64,
    playback_status: String,
    position: u64,
}

fn get_dict_string(dict: &zvariant::Dict, key: &str) -> Option<String> {
    for pair in dict.iter() {
        if let Ok(k) = pair.0.downcast_ref::<String>() {
            if k == key {
                if let Ok(v) = pair.1.downcast_ref::<String>() {
                    return Some(v.clone());
                }
                if let Ok(arr) = pair.1.downcast_ref::<zvariant::Array>() {
                    for item in arr.iter() {
                        if let Ok(s) = item.downcast_ref::<String>() {
                            return Some(s.clone());
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_dict_u64(dict: &zvariant::Dict, key: &str) -> Option<u64> {
    for pair in dict.iter() {
        if let Ok(k) = pair.0.downcast_ref::<String>() {
            if k == key {
                if let Ok(v) = pair.1.downcast_ref::<u64>() {
                    return Some(v);
                }
                if let Ok(v) = pair.1.downcast_ref::<i64>() {
                    return Some(v as u64);
                }
                if let Ok(v) = pair.1.downcast_ref::<u32>() {
                    return Some(v as u64);
                }
                if let Ok(v) = pair.1.downcast_ref::<i32>() {
                    return Some(v as u64);
                }
            }
        }
    }
    None
}

fn strip_bus_prefix(name: &str) -> String {
    name.strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(name).to_string()
}
