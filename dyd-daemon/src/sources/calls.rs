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

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dyd_types::{CallPayload, IpcEnvelope, IpcEvent};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, watch};
use zbus::interface;

const RING_HOLD: Duration = Duration::from_millis(1500);
const NOTIFICATION_HOLD: Duration = Duration::from_secs(45);
/// A capture stream that appears this long after the ringer started is the answered call.
const MIC_ANSWER_DELAY: Duration = Duration::from_millis(800);
/// The ringer window closed, but call audio is still playing: the call was picked up.
const WINDOW_LOSS_ANSWER: Duration = Duration::from_millis(1500);
/// Answered, and every live signal (audio, window, notification, microphone) is gone.
const QUIET_END: Duration = Duration::from_secs(4);

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
    /// Keeps an answered call on the island after the ringer notification is gone.
    latch: Option<View>,
    answered: bool,
    started_at: f64,
    /// Set when a ringing window disappears while the call may still be live.
    window_lost_at: Option<Instant>,
    mic_live: bool,
    quiet_since: Option<Instant>,
    ended_at: Option<Instant>,
    shown: Option<View>,
    shown_phase: String,
    shown_started: f64,
}

pub struct CallHub {
    tx: broadcast::Sender<IpcEnvelope>,
    slots: Mutex<Slots>,
    bus: Mutex<Option<zbus::Connection>>,
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
                latch: None,
                answered: false,
                started_at: 0.0,
                window_lost_at: None,
                mic_live: false,
                quiet_since: None,
                ended_at: None,
                shown: None,
                shown_phase: String::new(),
                shown_started: 0.0,
            }),
            bus: Mutex::new(None),
        })
    }

    pub fn attach_bus(&self, conn: zbus::Connection) {
        match self.bus.lock() {
            Ok(mut bus) => *bus = Some(conn),
            Err(poisoned) => *poisoned.into_inner() = Some(conn),
        }
    }

    /// Tell the shell extension to accept, reject, or mute the live call banner.
    pub async fn emit_control(&self, action: &str) -> zbus::Result<()> {
        let conn = match self.bus.lock() {
            Ok(bus) => bus.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        let Some(conn) = conn else {
            tracing::warn!("call control skipped; session bus is not ready");
            return Ok(());
        };
        let message =
            zbus::Message::signal("/app/dyland/Island", "app.dyland.Island", "CallControl")?
                .build(&action)?;
        conn.send(&message).await?;
        tracing::info!(action, "call control sent");
        Ok(())
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.tx.clone()
    }

    pub fn set_window(&self, active: bool, app_name: &str, summary: &str) {
        let view = active.then(|| view_for(app_name, summary, None));
        self.update(|slots| {
            if view.is_some() && suppress_after_end(slots) {
                return;
            }
            if view.is_none() && slots.window.is_some() && !slots.answered {
                slots.window_lost_at = Some(Instant::now());
            }
            if view.is_some() {
                slots.window_lost_at = None;
            }
            slots.window = view;
        });
    }

    /// The shell pressed Answer, or the call UI is now the connected call.
    pub fn mark_answered(&self, app_name: &str, summary: &str) {
        self.update(|slots| {
            if slots.latch.is_none() {
                slots.latch = Some(
                    slots
                        .shown
                        .clone()
                        .or_else(|| slots.notification.clone())
                        .or_else(|| slots.window.clone())
                        .or_else(|| slots.audio.clone())
                        .unwrap_or_else(|| view_for(app_name, summary, None)),
                );
            }
            answer_slots(slots);
        });
    }

    pub fn call_shown(&self) -> bool {
        self.with_slots(|slots| slots.shown.is_some())
    }

    pub fn call_answered(&self) -> bool {
        self.with_slots(|slots| slots.answered)
    }

    /// Promote a closed ringer window, and drop an answered call whose signals are gone.
    pub fn poll_call_progress(&self, mic_live: Option<bool>) {
        self.update(|slots| {
            if let Some(live) = mic_live {
                slots.mic_live = live;
            }
            consider_window_loss(slots);
            consider_quiet(slots);
        });
    }

    fn with_slots<T>(&self, read: impl FnOnce(&Slots) -> T) -> T {
        let slots = match self.slots.lock() {
            Ok(slots) => slots,
            Err(poisoned) => poisoned.into_inner(),
        };
        read(&slots)
    }

    /// The call was declined or the in-call controls went away.
    pub fn mark_ended(&self) {
        self.update(|slots| {
            slots.answered = false;
            slots.started_at = 0.0;
            slots.window_lost_at = None;
            slots.mic_live = false;
            slots.quiet_since = None;
            slots.latch = None;
            slots.ended_at = Some(Instant::now());
            slots.notification = None;
            slots.notification_key.clear();
            slots.notification_gen = slots.notification_gen.wrapping_add(1);
            slots.window = None;
            slots.audio = None;
        });
    }

    fn set_audio(&self, view: Option<View>) {
        self.update(|slots| {
            if view.is_some() && suppress_after_end(slots) {
                return;
            }
            slots.audio = view;
        });
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

fn unix_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn suppress_after_end(slots: &Slots) -> bool {
    slots
        .ended_at
        .is_some_and(|ended| ended.elapsed() < Duration::from_secs(2))
}

fn answer_slots(slots: &mut Slots) {
    slots.ended_at = None;
    slots.window_lost_at = None;
    slots.quiet_since = None;
    slots.answered = true;
    if slots.started_at <= 0.0 {
        slots.started_at = unix_secs();
    }
    if slots.latch.is_none() {
        slots.latch = slots
            .shown
            .clone()
            .or_else(|| slots.notification.clone())
            .or_else(|| slots.window.clone())
            .or_else(|| slots.audio.clone());
    }
}

/// The incoming-call window went away while playback is still up. That is an
/// answered WhatsApp or Snapchat call whose page title dropped the word incoming.
fn consider_window_loss(slots: &mut Slots) {
    let Some(lost) = slots.window_lost_at else {
        return;
    };
    if slots.answered || slots.window.is_some() {
        slots.window_lost_at = None;
        return;
    }
    if slots.audio.is_none() || lost.elapsed() < WINDOW_LOSS_ANSWER {
        return;
    }
    answer_slots(slots);
}

fn consider_quiet(slots: &mut Slots) {
    if !slots.answered {
        slots.quiet_since = None;
        return;
    }
    let live = slots.audio.is_some()
        || slots.window.is_some()
        || slots.notification.is_some()
        || slots.mic_live;
    if live {
        slots.quiet_since = None;
        return;
    }
    let since = *slots.quiet_since.get_or_insert_with(Instant::now);
    if since.elapsed() < QUIET_END {
        return;
    }
    slots.answered = false;
    slots.started_at = 0.0;
    slots.latch = None;
    slots.quiet_since = None;
    slots.window_lost_at = None;
}

fn microphone_means_answered(
    ring_age: Duration,
    baseline: &HashSet<String>,
    current: &[String],
) -> bool {
    if ring_age < MIC_ANSWER_DELAY {
        return false;
    }
    current.iter().any(|key| !baseline.contains(key))
}

fn publish(slots: &mut Slots, tx: &broadcast::Sender<IpcEnvelope>) {
    if !slots.answered {
        slots.latch = None;
        slots.started_at = 0.0;
    }
    let next = slots
        .window
        .clone()
        .or_else(|| slots.notification.clone())
        .or_else(|| slots.audio.clone())
        .or_else(|| slots.latch.clone());
    let phase = if next.is_some() && slots.answered {
        "active"
    } else if next.is_some() {
        "ringing"
    } else {
        ""
    };
    let started_at = if phase == "active" {
        slots.started_at
    } else {
        0.0
    };
    if next == slots.shown
        && slots.shown_phase == phase
        && (started_at - slots.shown_started).abs() < 0.001
    {
        return;
    }
    if next.is_none() {
        slots.window_lost_at = None;
    }
    let starting = slots.shown.is_none() && next.is_some();
    let ending = slots.shown.is_some() && next.is_none();
    let picked_up = slots.shown.is_some() && phase == "active" && slots.shown_phase != "active";
    slots.shown = next.clone();
    slots.shown_phase = phase.to_string();
    slots.shown_started = started_at;
    if starting {
        tracing::info!("incoming call shown on the island");
    } else if picked_up {
        tracing::info!("call answered; island timer started");
    } else if ending {
        tracing::info!("incoming call cleared");
    }
    let payload = match next {
        Some(view) => CallPayload {
            active: true,
            app_name: view.app_name,
            summary: view.summary,
            body: view.body,
            phase: phase.to_string(),
            started_at,
        },
        None => CallPayload {
            active: false,
            app_name: String::new(),
            summary: String::new(),
            body: None,
            phase: String::new(),
            started_at: 0.0,
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

    fn set_call_answered(&self, app_name: &str, summary: &str) {
        tracing::info!(app_name, summary, "call answered");
        self.hub.mark_answered(app_name, summary);
    }

    fn set_call_ended(&self, reason: &str) {
        tracing::info!(reason, "call ended from the shell");
        self.hub.mark_ended();
    }
}

pub async fn serve_island(conn: zbus::Connection, hub: Arc<CallHub>) -> zbus::Result<()> {
    hub.attach_bus(conn.clone());
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
    let mut watch = RingerWatch::default();
    let hold = tokio::time::sleep(Duration::from_secs(24 * 60 * 60));
    tokio::pin!(hold);

    if let Some(delay) = apply_streams(&hub, &mut watch).await {
        hold.as_mut().reset(tokio::time::Instant::now() + delay);
    }

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                let _ = child.kill().await;
                return Ok(());
            }
            _ = &mut hold => {
                if let Some(delay) = apply_streams(&hub, &mut watch).await {
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
                if !line.contains("sink-input") && !line.contains("source-output") {
                    continue;
                }
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        let _ = child.kill().await;
                        return Ok(());
                    }
                    _ = tokio::time::sleep(Duration::from_millis(200)) => {}
                }
                if let Some(delay) = apply_streams(&hub, &mut watch).await {
                    hold.as_mut().reset(tokio::time::Instant::now() + delay);
                }
            }
        }
    }
}

#[derive(Default)]
struct RingerWatch {
    pending_key: String,
    pending_since: Option<Instant>,
    mic_baseline: Option<HashSet<String>>,
    ring_started: Option<Instant>,
}

async fn apply_streams(hub: &CallHub, watch: &mut RingerWatch) -> Option<Duration> {
    let playback_delay = match list_sink_inputs().await {
        Ok(hits) => apply_playback(hub, watch, &hits),
        Err(err) => {
            tracing::debug!(error = %err, "could not list playback streams");
            None
        }
    };
    let mics = match list_source_outputs().await {
        Ok(mics) => Some(mics),
        Err(err) => {
            tracing::debug!(error = %err, "could not list capture streams");
            None
        }
    };
    let progress_delay = note_microphones(hub, watch, mics.as_deref());
    match (playback_delay, progress_delay) {
        (Some(playback), Some(progress)) => Some(playback.min(progress)),
        (Some(delay), None) | (None, Some(delay)) => Some(delay),
        (None, None) => None,
    }
}

fn apply_playback(hub: &CallHub, watch: &mut RingerWatch, hits: &[StreamHit]) -> Option<Duration> {
    let immediate = hits.iter().find(|hit| hit.strength == Strength::Immediate);
    if let Some(hit) = immediate {
        watch.pending_key.clear();
        watch.pending_since = None;
        hub.set_audio(Some(hit.view.clone()));
        return None;
    }
    let sustained = hits.iter().find(|hit| hit.strength == Strength::Sustained);
    let Some(hit) = sustained else {
        watch.pending_key.clear();
        watch.pending_since = None;
        hub.set_audio(None);
        return None;
    };
    let key = format!("{}|{}", hit.view.app_name, hit.view.summary);
    if watch.pending_key != key {
        watch.pending_key = key;
        watch.pending_since = Some(Instant::now());
    }
    let elapsed = watch
        .pending_since
        .map(|since| since.elapsed())
        .unwrap_or_default();
    if elapsed >= RING_HOLD {
        hub.set_audio(Some(hit.view.clone()));
        None
    } else {
        Some(RING_HOLD.saturating_sub(elapsed))
    }
}

fn note_microphones(
    hub: &CallHub,
    watch: &mut RingerWatch,
    mics: Option<&[String]>,
) -> Option<Duration> {
    if !hub.call_shown() {
        watch.mic_baseline = None;
        watch.ring_started = None;
        if mics.is_some() {
            hub.poll_call_progress(Some(false));
        }
        return None;
    }
    if let Some(mics) = mics {
        if watch.ring_started.is_none() {
            watch.ring_started = Some(Instant::now());
            watch.mic_baseline = Some(mics.iter().cloned().collect());
        } else if !hub.call_answered() {
            let age = watch
                .ring_started
                .map(|started| started.elapsed())
                .unwrap_or_default();
            let baseline = watch.mic_baseline.clone().unwrap_or_default();
            if microphone_means_answered(age, &baseline, mics) {
                tracing::info!("call answered; microphone opened");
                hub.mark_answered("", "");
            }
        }
        hub.poll_call_progress(Some(!mics.is_empty()));
    } else {
        hub.poll_call_progress(None);
    }
    if hub.call_shown() {
        Some(Duration::from_millis(500))
    } else {
        None
    }
}

pub async fn set_ringtone_muted(muted: bool) {
    let output = match Command::new("pactl")
        .args(["-f", "json", "list", "sink-inputs"])
        .output()
        .await
    {
        Ok(output) if output.status.success() => output,
        Ok(_) => {
            tracing::warn!("could not list playback streams to mute the ringtone");
            return;
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not list playback streams to mute the ringtone");
            return;
        }
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        tracing::warn!("playback stream list was not json");
        return;
    };
    let Some(items) = value.as_array() else {
        return;
    };
    let flag = if muted { "1" } else { "0" };
    let mut changed = 0_u32;
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
        if classify_playback(corked, &properties).is_none() {
            continue;
        }
        let Some(index) = item.get("index").and_then(json_index) else {
            continue;
        };
        match Command::new("pactl")
            .args(["set-sink-input-mute", &index, flag])
            .status()
            .await
        {
            Ok(status) if status.success() => changed += 1,
            Ok(_) => tracing::warn!(index, "could not mute a call stream"),
            Err(err) => tracing::warn!(error = %err, index, "could not mute a call stream"),
        }
    }
    tracing::info!(muted, streams = changed, "call ringtone mute updated");
}

fn json_index(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::String(text) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

async fn list_source_outputs() -> std::io::Result<Vec<String>> {
    let items = pulse_items("source-outputs").await?;
    let mut keys = Vec::new();
    for item in items {
        let properties = pulse_properties(&item);
        let Some(key) = classify_capture(&properties) else {
            continue;
        };
        let index = item
            .get("index")
            .and_then(json_index)
            .unwrap_or_else(|| "0".to_string());
        keys.push(format!("{index}:{key}"));
    }
    Ok(keys)
}

fn pulse_properties(item: &serde_json::Value) -> HashMap<String, String> {
    let mut properties = HashMap::new();
    if let Some(map) = item.get("properties").and_then(|value| value.as_object()) {
        for (key, value) in map {
            let text = match value {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            properties.insert(key.clone(), text);
        }
    }
    properties
}

async fn pulse_items(kind: &str) -> std::io::Result<Vec<serde_json::Value>> {
    let output = Command::new("pactl")
        .args(["-f", "json", "list", kind])
        .output()
        .await?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!("pactl list {kind} failed")));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| std::io::Error::other(err.to_string()))?;
    Ok(value.as_array().cloned().unwrap_or_default())
}

/// A microphone opened by a calling app or the browser. Muted streams still count.
fn classify_capture(properties: &HashMap<String, String>) -> Option<String> {
    let app = prop(properties, "application.name");
    let binary = prop(properties, "application.process.binary");
    let process = prop(properties, "application.process.name");
    let media = prop(properties, "media.name");
    let ident = prop(properties, "application.id");
    let blob = format!("{app} {binary} {process} {media} {ident}");
    let lower = blob.to_lowercase();
    if lower.contains("voice message")
        || lower.contains("audio message")
        || lower.contains("voice note")
        || lower.contains("ptt")
    {
        return None;
    }
    let call_app =
        known_app_label(&blob).is_some() || is_browser(&format!("{binary} {process} {app}"));
    if !call_app {
        return None;
    }
    let pid = prop(properties, "application.process.id");
    Some(format!("{pid}|{binary}|{media}"))
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
                assert_eq!(payload.phase, "");
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[test]
    fn answered_call_shows_a_timer_until_it_ends() {
        let hub = CallHub::new();
        let mut rx = hub.event_broadcast().subscribe();
        hub.set_window(true, "WhatsApp", "Maamaa");
        let start = rx.try_recv().expect("ringing");
        match start.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert_eq!(payload.phase, "ringing");
                assert_eq!(payload.started_at, 0.0);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
        hub.mark_answered("WhatsApp", "Maamaa");
        let answered = rx.try_recv().expect("answered");
        match answered.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert!(payload.active);
                assert_eq!(payload.phase, "active");
                assert_eq!(payload.summary, "Maamaa");
                assert!(payload.started_at > 0.0);
            }
            other => panic!("unexpected payload: {other:?}"),
        };
        hub.set_window(false, "", "");
        assert!(
            rx.try_recv().is_err(),
            "an answered call stays up after the ringer window closes"
        );
        hub.mark_ended();
        let end = rx.try_recv().expect("ended");
        match end.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert!(!payload.active);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[test]
    fn a_new_microphone_after_the_ringer_starts_means_answered() {
        let baseline = std::collections::HashSet::from(["1:old".to_string()]);
        let fresh = ["2:new".to_string()];
        assert!(!microphone_means_answered(
            Duration::from_millis(100),
            &baseline,
            &fresh,
        ));
        assert!(microphone_means_answered(
            Duration::from_secs(1),
            &baseline,
            &fresh,
        ));
        assert!(!microphone_means_answered(
            Duration::from_secs(1),
            &baseline,
            &["1:old".to_string()],
        ));
    }

    #[test]
    fn browser_microphone_is_a_call_capture_and_voice_notes_are_not() {
        let mic = classify_capture(&props(&[
            ("application.name", "WhatsApp"),
            ("application.process.binary", "chrome"),
            ("application.process.id", "42"),
            ("media.name", "WebRTC"),
        ]));
        assert_eq!(mic.as_deref(), Some("42|chrome|WebRTC"));
        assert!(classify_capture(&props(&[
            ("application.name", "Spotify"),
            ("application.process.binary", "spotify"),
            ("media.name", "Spotify"),
        ]))
        .is_none());
        assert!(classify_capture(&props(&[
            ("application.name", "WhatsApp"),
            ("media.name", "Voice message"),
        ]))
        .is_none());
    }

    #[test]
    fn closed_ringer_window_with_audio_becomes_an_answered_call() {
        let hub = CallHub::new();
        let mut rx = hub.event_broadcast().subscribe();
        hub.set_window(true, "WhatsApp", "Incoming voice call from Elroberto");
        let _ = rx.try_recv();
        hub.set_audio(Some(View {
            app_name: "WhatsApp".into(),
            summary: "Incoming call".into(),
            body: None,
        }));
        let _ = rx.try_recv();
        hub.set_window(false, "", "");
        let still = rx.try_recv().expect("audio remains");
        match still.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert_eq!(payload.phase, "ringing");
            }
            other => panic!("unexpected payload: {other:?}"),
        }
        {
            let mut slots = hub.slots.lock().expect("slots");
            slots.window_lost_at = Instant::now().checked_sub(Duration::from_secs(2));
        }
        hub.poll_call_progress(None);
        let answered = rx.try_recv().expect("answered");
        match answered.payload {
            dyd_types::IpcPayload::Event(IpcEvent::CallChanged(payload)) => {
                assert!(payload.active);
                assert_eq!(payload.phase, "active");
                assert!(payload.started_at > 0.0);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[test]
    fn a_fresh_window_loss_without_audio_stays_unanswered() {
        let mut slots = Slots {
            audio: None,
            window: None,
            notification: None,
            notification_key: String::new(),
            notification_gen: 0,
            latch: None,
            answered: false,
            started_at: 0.0,
            window_lost_at: Instant::now().checked_sub(Duration::from_secs(3)),
            mic_live: false,
            quiet_since: None,
            ended_at: None,
            shown: None,
            shown_phase: String::new(),
            shown_started: 0.0,
        };
        consider_window_loss(&mut slots);
        assert!(!slots.answered);
        slots.audio = Some(View {
            app_name: "WhatsApp".into(),
            summary: "Incoming call".into(),
            body: None,
        });
        slots.window_lost_at = Some(Instant::now());
        consider_window_loss(&mut slots);
        assert!(
            !slots.answered,
            "a brand-new window loss is still the ringer"
        );
    }
}
