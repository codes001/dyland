use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcEvent {
    Pong(IpcPongPayload),
    MediaChanged(MediaChangedPayload),
    NotificationReceived(NotificationPayload),
    BatteryChanged(BatteryPayload),
    ConnectionStatus(ConnectionStatusPayload),
    ActivityQueued(ActivityPayload),
    ActivityDequeued(ActivityPayload),
    ActivityChanged(ActivityPayload),
    QueueEmptied,
    FullscreenChanged(FullscreenPayload),
    DndChanged(DndPayload),
    ConfigChanged(ConfigChangedPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcCommand {
    Ping,
    GetState,
    DismissActivity { activity_id: String },
    CycleNext,
    CyclePrev,
    MediaPlayPause { player: String },
    MediaNext { player: String },
    MediaPrevious { player: String },
    MediaSeek { player: String, position_us: u64 },
    MediaSetVolume { player: String, volume: f64 },
    NotificationAction { id: u32, action_key: String, reply_text: Option<String> },
    NotificationClose { id: u32 },
    SetDnd { enabled: bool },
    SetVolume { volume: f64 },
    SetMute { muted: bool },
    SetBrightness { brightness: f64 },
    SetBluetoothPowered { powered: bool },
    GetConfig,
    UpdateConfig { changes: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEnvelope {
    pub id: Uuid,
    pub timestamp: f64,
    pub kind: IpcDirection,
    #[serde(flatten)]
    pub payload: IpcPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcDirection {
    Event,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IpcPayload {
    Event(IpcEvent),
    Command(IpcCommand),
}

impl IpcEnvelope {
    pub fn event(event: IpcEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            kind: IpcDirection::Event,
            payload: IpcPayload::Event(event),
        }
    }

    pub fn command(command: IpcCommand) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            kind: IpcDirection::Command,
            payload: IpcPayload::Command(command),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = serde_json::to_vec(self).expect("IpcEnvelope serialization failed");
        buf.push(b'\n');
        buf
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcPongPayload {
    pub status: String,
    pub uptime_secs: u64,
    pub version: String,
    pub connected_clients: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaChangedPayload {
    pub player: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub length: u64,
    pub position: u64,
    pub playback_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub id: u32,
    pub app_name: String,
    pub app_icon: Option<String>,
    pub summary: String,
    pub body: Option<String>,
    pub urgency: String,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryPayload {
    pub percentage: f64,
    pub state: String,
    pub time_to_empty: Option<u64>,
    pub time_to_full: Option<u64>,
    pub is_present: bool,
}

impl BatteryPayload {
    pub fn sanitize(mut self) -> Self {
        if !self.percentage.is_finite() {
            self.percentage = 0.0;
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatusPayload {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityPayload {
    pub activity_id: String,
    pub source: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullscreenPayload {
    pub is_fullscreen: bool,
    pub monitor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DndPayload {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangedPayload {
    pub changes: serde_json::Value,
}
