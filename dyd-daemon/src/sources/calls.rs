//! Incoming-call detection for the island.
//!
//! A ringing call often never becomes a freedesktop notification: WhatsApp,
//! Snapchat, and the other web clients draw the ringer inside their own window
//! when that window is focused. This module treats a call as active when any
//! of three signals say so:
//!
//! - a notification whose text, category, or actions describe a call
//! - a window the shell extension reports (title such as "Incoming voice call")
//! - playback from a known calling app that keeps going long enough to be a
//!   ringtone rather than a message ping, or any stream with the phone role

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dyd_types::{CallPayload, IpcEnvelope, IpcEvent};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, watch};
use zbus::interface;

const RING_HOLD: Duration = Duration::from_millis(1500);
const NOTIFICATION_HOLD: Duration = Duration::from_secs(45);

const CALL_PHRASES: &[&str] = &[
    "incoming call",
    "incoming voice",
    "incoming video",
    "voice call",
    "video call",
    "is calling",
    "calling you",
    "ringing",
    "webrtc",
    "ongoing call",
    "on a call",
];

const CALL_APPS: &[(&str, &str)] = &[
    ("whatsapp", "WhatsApp"),
    ("snapchat", "Snapchat"),
    ("telegram", "Telegram"),
    ("signal", "Signal"),
    ("discord", "Discord"),
    ("slack", "Slack"),
    ("zoom", "Zoom"),
    ("skype", "Skype"),
    ("viber", "Viber"),
    ("teams", "Teams"),
    ("messenger", "Messenger"),
    ("caprine", "Messenger"),
    ("element", "Element"),
    ("wechat", "WeChat"),
    ("webex", "Webex"),
    ("threema", "Threema"),
];

const BROWSERS: &[&str] = &[
    "chrome",
    "chromium",
    "firefox",
    "brave",
    "opera",
    "msedge",
    "microsoft-edge",
    "epiphany",
    "safari",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct View {
    app_name: String,
    summary: String,
    body: Option<String>,
}

struct Slots {
    audio: Option<View>,
    window: Option<View>,
    notification: Option<View>,
    notification_key: String,
    notification_gen: u64,
    shown: Option<View>,
}

pub struct CallHub {
    tx: broadcast::Sender<IpcEnvelope>,
    slots: Mutex<Slots>,
}

impl CallHub {
    pub fn new() -> Arc<Self> {
        let (tx, _) = broadcast::channel(32);
        Arc::new(Self {
            tx,
            slots: Mutex::new(Slots {
                audio: None,
                window: None,
                notification: None,
                notification_key: String::new(),
                notification_gen: 0,
                shown: None,
            }),
        })
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.tx.clone()
    }

    pub fn set_window(&self, active: bool, app_name: &str, summary: &str) {
        let view = active.then(|| view_for(app_name, summary, None));
        self.update(|slots| slots.window = view);
    }

    fn set_audio(&self, view: Option<View>) {
        self.update(|slots| slots.audio = view);
    }

    /// Record a call notification and drop it if nothing ends it explicitly.
    pub fn raise_notification(
        self: &Arc<Self>,
        key: String,
        app_name: &str,
        summary: &str,
        body: Option<String>,
    ) {
        let view = view_for(app_name, summary, body);
        let gen = {
            let mut slots = match self.slots.lock() {
                Ok(slots) => slots,
                Err(poisoned) => poisoned.into_inner(),
            };
            slots.notification_key = key;
            slots.notification = Some(view);
            slots.notification_gen = slots.notification_gen.wrapping_add(1);
            let gen = slots.notification_gen;
            publish(&mut slots, &self.tx);
            gen
        };
        let hub = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(NOTIFICATION_HOLD).await;
            hub.expire_notification(gen);
        });
    }

    pub fn clear_notification(&self, key: &str) {
        self.update(|slots| {
            if slots.notification_key == key {
                slots.notification = None;
                slots.notification_key.clear();
            }
        });
    }

    fn expire_notification(&self, gen: u64) {
        self.update(|slots| {
            if slots.notification_gen == gen {
                slots.notification = None;
                slots.notification_key.clear();
            }
        });
    }

    fn update(&self, apply: impl FnOnce(&mut Slots)) -> Option<u64> {
        let mut slots = match self.slots.lock() {
            Ok(slots) => slots,
            Err(poisoned) => poisoned.into_inner(),
        };
        apply(&mut slots);
        publish(&mut slots, &self.tx);
        Some(slots.notification_gen)
    }
}

fn view_for(app_name: &str, summary: &str, body: Option<String>) -> View {
    let app_name = app_name.trim();
    let summary = summary.trim();
    let app_name = if app_name.is_empty() {
        known_app_label(&format!("{summary} {}", body.as_deref().unwrap_or("")))
            .unwrap_or("Call")
            .to_string()
    } else {
        app_name.to_string()
    };
    let summary = if summary.is_empty() {
        "Incoming call".to_string()
    } else {
        summary.to_string()
    };
    View {
        app_name,
        summary,
        body: body.and_then(|text| {
            let text = text.trim().to_string();
            (!text.is_empty()).then_some(text)
        }),
    }
}

fn publish(slots: &mut Slots, tx: &broadcast::Sender<IpcEnvelope>) {
    let next = slots
        .window
        .clone()
        .or_else(|| slots.notification.clone())
        .or_else(|| slots.audio.clone());
    if next == slots.shown {
        return;
    }
    let starting = slots.shown.is_none() && next.is_some();
    let ending = slots.shown.is_some() && next.is_none();
    slots.shown = next.clone();
    if starting {
        tracing::info!("incoming call shown on the island");
    } else if ending {
        tracing::info!("incoming call cleared");
    }
    let payload = match next {
        Some(view) => CallPayload {
            active: true,
            app_name: view.app_name,
            summary: view.summary,
            body: view.body,
        },
        None => CallPayload {
            active: false,
            app_name: String::new(),
            summary: String::new(),
            body: None,
        },
    };
    let _ = tx.send(IpcEnvelope::event(IpcEvent::CallChanged(payload)));
}

pub fn text_mentions_call(text: &str) -> bool {
    let lower = text.to_lowercase();
    CALL_PHRASES.iter().any(|phrase| lower.contains(phrase))
}

pub fn known_app_label(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    CALL_APPS
        .iter()
        .find(|(token, _)| lower.contains(token))
        .map(|(_, label)| *label)
}

fn is_browser(text: &str) -> bool {
    let lower = text.to_lowercase();
    BROWSERS.iter().any(|token| lower.contains(token))
}

pub struct NotifyFields<'a> {
    pub app_name: &'a str,
    pub summary: &'a str,
    pub body: &'a str,
    pub category: &'a str,
    pub actions: &'a str,
}

pub fn notification_is_call(fields: &NotifyFields<'_>) -> bool {
    let category = fields.category.to_lowercase();
    if category.starts_with("call") {
        return true;
    }
    let blob = format!(
        "{} {} {} {}",
        fields.app_name, fields.summary, fields.body, fields.actions
    );
    if text_mentions_call(&blob) {
        return true;
    }
    let actions = fields.actions.to_lowercase();
    let answers = ["answer", "accept", "decline", "reject"]
        .iter()
        .any(|word| actions.contains(word));
    answers && (blob.to_lowercase().contains("call") || known_app_label(&blob).is_some())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strength {
    Immediate,
    Sustained,
}

#[derive(Clone, Debug)]
struct StreamHit {
    strength: Strength,
    view: View,
}

fn classify_playback(corked: bool, properties: &HashMap<String, String>) -> Option<StreamHit> {
    if corked {
        return None;
    }
    let app = prop(properties, "application.name");
    let binary = prop(properties, "application.process.binary");
    let process = prop(properties, "application.process.name");
    let media = prop(properties, "media.name");
    let role = prop(properties, "media.role").to_lowercase();
    let ident = prop(properties, "application.id");
    let blob = format!("{app} {binary} {process} {media} {ident} {role}");
    let lower = blob.to_lowercase();
    if lower.contains("voice message")
        || lower.contains("audio message")
        || lower.contains("voice note")
        || lower.contains("ptt")
    {
        return None;
    }

    let known = known_app_label(&blob);
    let browser = is_browser(&format!("{binary} {process} {app}"));
    let phrase = text_mentions_call(&blob);
    let phone = role == "phone" || role == "communication";
    let event = role == "event" || role == "notification" || role == "alert";

    let strength = if phone || phrase || (known.is_some() && event && !browser) {
        Strength::Immediate
    } else if known.is_some() && (!browser || known_app_label(&format!("{app} {media}")).is_some())
    {
        Strength::Sustained
    } else {
        return None;
    };

    let app_name = known
        .unwrap_or(if app.is_empty() { "Call" } else { app })
        .to_string();
    let summary = if phrase && !media.is_empty() && text_mentions_call(media) {
        media.to_string()
    } else {
        "Incoming call".to_string()
    };
    Some(StreamHit {
        strength,
        view: View {
            app_name,
            summary,
            body: None,
        },
    })
}

fn prop<'a>(properties: &'a HashMap<String, String>, key: &str) -> &'a str {
    properties.get(key).map(String::as_str).unwrap_or("")
}

struct IslandIface {
    hub: Arc<CallHub>,
}

#[interface(name = "app.dyland.Island")]
impl IslandIface {
    fn set_incoming_call(&self, app_name: &str, summary: &str, active: bool) {
        self.hub.set_window(active, app_name, summary);
    }
}

pub async fn serve_island(conn: zbus::Connection, hub: Arc<CallHub>) -> zbus::Result<()> {
    conn.object_server()
        .at("/app/dyland/Island", IslandIface { hub })
        .await?;
    conn.request_name("app.dyland").await?;
    tracing::info!("incoming-call API is on the session bus");
    Ok(())
}

pub async fn watch_audio(hub: Arc<CallHub>, mut shutdown_rx: watch::Receiver<()>) {
    loop {
        let session_shutdown = shutdown_rx.clone();
        tokio::select! {
            _ = shutdown_rx.changed() => return,
            result = audio_session(hub.clone(), session_shutdown) => {
                if result.is_err() {
                    tracing::warn!("call audio monitor stopped; retrying");
                }
                tokio::select! {
                    _ = shutdown_rx.changed() => return,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                }
            }
        }
    }
}

async fn audio_session(
    hub: Arc<CallHub>,
    mut shutdown_rx: watch::Receiver<()>,
) -> std::io::Result<()> {
    let mut child = Command::new("pactl")
        .arg("subscribe")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("pactl subscribe produced no output"))?;
    let mut lines = BufReader::new(stdout).lines();
    let mut pending_key = String::new();
    let mut pending_since: Option<Instant> = None;
    let hold = tokio::time::sleep(Duration::from_secs(24 * 60 * 60));
    tokio::pin!(hold);

    if let Some(delay) = apply_streams(&hub, &mut pending_key, &mut pending_since).await {
        hold.as_mut().reset(tokio::time::Instant::now() + delay);
    }

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                let _ = child.kill().await;
                return Ok(());
            }
            _ = &mut hold => {
                if let Some(delay) = apply_streams(&hub, &mut pending_key, &mut pending_since).await {
                    hold.as_mut().reset(tokio::time::Instant::now() + delay);
                } else {
                    hold.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(24 * 60 * 60));
                }
            }
            line = lines.next_line() => {
                let line = match line {
                    Ok(Some(line)) => line,
                    Ok(None) => return Err(std::io::Error::other("pactl subscribe ended")),
                    Err(err) => return Err(err),
                };
                if !line.contains("sink-input") {
                    continue;
                }
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        let _ = child.kill().await;
                        return Ok(());
                    }
                    _ = tokio::time::sleep(Duration::from_millis(200)) => {}
                }
                if let Some(delay) = apply_streams(&hub, &mut pending_key, &mut pending_since).await {
                    hold.as_mut().reset(tokio::time::Instant::now() + delay);
                }
            }
        }
    }
}

async fn apply_streams(
    hub: &CallHub,
    pending_key: &mut String,
    pending_since: &mut Option<Instant>,
) -> Option<Duration> {
    let hits = match list_sink_inputs().await {
        Ok(hits) => hits,
        Err(err) => {
            tracing::debug!(error = %err, "could not list playback streams");
            return None;
        }
    };
    let immediate = hits.iter().find(|hit| hit.strength == Strength::Immediate);
    if let Some(hit) = immediate {
        pending_key.clear();
        *pending_since = None;
        hub.set_audio(Some(hit.view.clone()));
        return None;
    }
    let sustained = hits.iter().find(|hit| hit.strength == Strength::Sustained);
    let Some(hit) = sustained else {
        pending_key.clear();
        *pending_since = None;
        hub.set_audio(None);
        return None;
    };
    let key = format!("{}|{}", hit.view.app_name, hit.view.summary);
    if pending_key != &key {
        *pending_key = key;
        *pending_since = Some(Instant::now());
    }
    let elapsed = pending_since
        .map(|since| since.elapsed())
        .unwrap_or_default();
    if elapsed >= RING_HOLD {
        hub.set_audio(Some(hit.view.clone()));
        None
    } else {
        Some(RING_HOLD.saturating_sub(elapsed))
    }
}

async fn list_sink_inputs() -> std::io::Result<Vec<StreamHit>> {
    let output = Command::new("pactl")
        .args(["-f", "json", "list", "sink-inputs"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(std::io::Error::other("pactl list sink-inputs failed"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| std::io::Error::other(err.to_string()))?;
    let Some(items) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::new();
    for item in items {
        let corked = item
            .get("corked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mut properties = HashMap::new();
        if let Some(map) = item.get("properties").and_then(|v| v.as_object()) {
            for (key, value) in map {
                let text = match value {
                    serde_json::Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                properties.insert(key.clone(), text);
            }
        }
        if let Some(hit) = classify_playback(corked, &properties) {
            hits.push(hit);
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn whatsapp_voice_notification_is_a_call() {
        let fields = NotifyFields {
            app_name: "WhatsApp",
            summary: "Ada",
            body: "Incoming voice call",
            category: "",
            actions: "Answer Decline",
        };
        assert!(notification_is_call(&fields));
    }

    #[test]
    fn ordinary_chat_notification_is_not_a_call() {
        let fields = NotifyFields {
            app_name: "WhatsApp",
            summary: "Ada",
            body: "see you at 6",
            category: "im.received",
            actions: "",
        };
        assert!(!notification_is_call(&fields));
    }

    #[test]
    fn call_category_is_a_call() {
        let fields = NotifyFields {
            app_name: "Phone",
            summary: "Ada",
            body: "",
            category: "call.incoming",
            actions: "",
        };
        assert!(notification_is_call(&fields));
    }

    #[test]
    fn phone_role_rings_immediately() {
        let hit = classify_playback(
            false,
            &props(&[
                ("application.name", "Chromium"),
                ("application.process.binary", "chrome"),
                ("media.name", "WebRTC"),
                ("media.role", "phone"),
            ]),
        )
        .expect("phone stream");
        assert_eq!(hit.strength, Strength::Immediate);
    }

    #[test]
    fn whatsapp_ringtone_is_sustained_not_immediate() {
        let hit = classify_playback(
            false,
            &props(&[
                ("application.name", "WhatsApp"),
                ("application.process.binary", "whatsapp"),
                ("media.name", "Playback"),
                ("media.role", "music"),
            ]),
        )
        .expect("ringtone");
        assert_eq!(hit.strength, Strength::Sustained);
        assert_eq!(hit.view.app_name, "WhatsApp");
        assert_eq!(hit.view.summary, "Incoming call");
    }

    #[test]
    fn snapchat_in_a_browser_tab_is_a_call_stream() {
        let hit = classify_playback(
            false,
            &props(&[
                ("application.name", "Snapchat"),
                ("application.process.binary", "firefox"),
                ("media.role", "music"),
            ]),
        )
        .expect("snapchat");
        assert_eq!(hit.view.app_name, "Snapchat");
        assert_eq!(hit.strength, Strength::Sustained);
    }

    #[test]
    fn music_and_voice_notes_are_not_calls() {
        assert!(classify_playback(
            false,
            &props(&[
                ("application.name", "Spotify"),
                ("media.name", "Spotify"),
                ("media.role", "music"),
            ]),
        )
        .is_none());
        assert!(classify_playback(
            false,
            &props(&[
                ("application.name", "WhatsApp"),
                ("media.name", "Voice message"),
                ("media.role", "music"),
            ]),
        )
        .is_none());
        assert!(classify_playback(
            true,
            &props(&[("application.name", "WhatsApp"), ("media.role", "phone")]),
        )
        .is_none());
    }

    #[test]
    fn hub_keeps_the_call_until_every_signal_drops() {
        let hub = CallHub::new();
        let mut rx = hub.event_broadcast().subscribe();
        hub.set_window(true, "WhatsApp", "Incoming voice call");
        let first = rx.try_recv().expect("call started");
        match first.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert!(payload.active);
                assert_eq!(payload.app_name, "WhatsApp");
                assert_eq!(payload.summary, "Incoming voice call");
            }
            other => panic!("unexpected payload: {other:?}"),
        }
        hub.set_audio(Some(View {
            app_name: "WhatsApp".into(),
            summary: "Incoming call".into(),
            body: None,
        }));
        assert!(rx.try_recv().is_err(), "same call must not republish");
        hub.set_window(false, "", "");
        let still = rx.try_recv().expect("audio remains");
        match still.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert!(payload.active);
                assert_eq!(payload.summary, "Incoming call");
            }
            other => panic!("unexpected payload: {other:?}"),
        }
        hub.set_audio(None);
        let end = rx.try_recv().expect("call ended");
        match end.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert!(!payload.active);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }
}
