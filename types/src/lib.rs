pub mod plugin;

pub use plugin::{PluginManifest, ValidationIssue, ValidationReport};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcEvent {
    Pong(IpcPongPayload),
    MediaChanged(MediaChangedPayload),
    NotificationReceived(NotificationPayload),
    CallChanged(CallPayload),
    BatteryChanged(BatteryPayload),
    WifiChanged(WifiPayload),
    BluetoothChanged(BluetoothPayload),
    VolumeChanged(VolumePayload),
    BrightnessChanged(BrightnessPayload),
    ConnectionStatus(ConnectionStatusPayload),
    ActivityQueued(ActivityPayload),
    ActivityDequeued(ActivityPayload),
    ActivityChanged(ActivityPayload),
    QueueEmptied,
    FullscreenChanged(FullscreenPayload),
    DndChanged(DndPayload),
    ConfigChanged(ConfigChangedPayload),
    PluginChanged(PluginHealthPayload),
    PluginActivity(ActivityPayload),
    PluginList(PluginListPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcCommand {
    Ping,
    GetState,
    DismissActivity {
        activity_id: String,
    },
    CycleNext,
    CyclePrev,
    MediaPlayPause {
        player: String,
    },
    MediaNext {
        player: String,
    },
    MediaPrevious {
        player: String,
    },
    MediaSeek {
        player: String,
        position_us: i64,
    },
    MediaSetVolume {
        player: String,
        volume: f64,
    },
    CyclePlayer {
        player: Option<String>,
    },
    NotificationAction {
        id: u32,
        action_key: String,
        reply_text: Option<String>,
    },
    NotificationClose {
        id: u32,
    },
    CallControl {
        action: String,
        #[serde(default)]
        muted: Option<bool>,
    },
    SetDnd {
        enabled: bool,
    },
    SetVolume {
        volume: f64,
    },
    SetMute {
        muted: bool,
    },
    SetBrightness {
        brightness: f64,
    },
    SetBluetoothPowered {
        powered: bool,
    },
    GetConfig,
    UpdateConfig {
        changes: serde_json::Value,
    },
    GetPlugins,
    GetPluginHealth {
        name: String,
    },
    InstallPlugin {
        path: String,
    },
    SetPluginEnabled {
        name: String,
        enabled: bool,
    },
    UninstallPlugin {
        name: String,
    },
    SetPluginConfig {
        name: String,
        config: serde_json::Value,
    },
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
pub struct CallPayload {
    pub active: bool,
    pub app_name: String,
    pub summary: String,
    pub body: Option<String>,
    /// `ringing` until the call is picked up, then `active`.
    #[serde(default)]
    pub phase: String,
    /// Unix seconds when the call was answered. `0` while it is still ringing.
    #[serde(default)]
    pub started_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationActionItem {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub id: u32,
    pub app_name: String,
    pub app_icon: Option<String>,
    pub summary: String,
    pub body: Option<String>,
    pub urgency: String,
    pub actions: Vec<NotificationActionItem>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryPayload {
    pub percentage: f64,
    pub state: String,
    pub time_to_empty: Option<u64>,
    pub time_to_full: Option<u64>,
    pub is_present: bool,
    pub health: Option<f64>,
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
pub struct WifiPayload {
    pub ssid: Option<String>,
    pub strength: u8,
    pub state: String,
    pub ip_address: Option<String>,
    pub frequency_mhz: Option<u32>,
    pub security: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub name: String,
    pub connected: bool,
    pub icon: Option<String>,
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothPayload {
    pub powered: bool,
    pub discoverable: bool,
    pub present: bool,
    pub devices: Vec<BluetoothDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumePayload {
    pub volume: f64,
    pub muted: bool,
    pub sink_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrightnessPayload {
    pub brightness: u8,
    pub has_backlight: bool,
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

/// Root application configuration (Epic 7). Every field is defaulted so a
/// missing or partial `config.json` still hydrates a complete, valid config.
/// Serialized to `$XDG_CONFIG_HOME/dyd/config.json` and mirrored to the UI in
/// `ConfigChanged` events. The daemon stores this as a `serde_json::Value` and
/// deep-merges updates, so unknown fields on disk are preserved across writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub accessibility: AccessibilityConfig,
    pub features: FeatureToggles,
    pub notifications: NotificationConfig,
    pub shortcuts: ShortcutConfig,
    /// Last-opened settings category, so the panel reopens where the user left off.
    pub last_category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GeneralConfig {
    pub start_on_login: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// `"dark"` (default), `"light"`, or `"system"`.
    pub theme: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
        }
    }
}

/// Accessibility overrides. `None` means "follow the system preference"
/// (`prefers-reduced-motion` / `prefers-contrast`); `Some(bool)` is an explicit
/// user override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AccessibilityConfig {
    pub reduce_motion: Option<bool>,
    pub high_contrast: Option<bool>,
    pub font_scale: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppNotificationRule {
    pub allow: bool,
    pub show_silently: bool,
    pub queue_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NotificationConfig {
    pub dnd_enabled: bool,
    pub app_rules: HashMap<String, AppNotificationRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutConfig {
    pub bindings: HashMap<String, String>,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert("expand".to_string(), "Space".to_string());
        bindings.insert("collapse".to_string(), "Escape".to_string());
        bindings.insert("prev_activity".to_string(), "ArrowLeft".to_string());
        bindings.insert("next_activity".to_string(), "ArrowRight".to_string());
        bindings.insert("toggle_dnd".to_string(), "d".to_string());
        bindings.insert("open_settings".to_string(), "s".to_string());
        Self { bindings }
    }
}

/// Per-feature enable flags. All default to `true`. Disabling hides a feature
/// in the UI and persists the preference; the daemon's source listener keeps
/// running (runtime start/stop is deferred — see deferred-work.md).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureToggles {
    pub battery: bool,
    pub wifi: bool,
    pub bluetooth: bool,
    pub volume: bool,
    pub brightness: bool,
    pub media: bool,
    pub notifications: bool,
}

impl Default for FeatureToggles {
    fn default() -> Self {
        Self {
            battery: true,
            wifi: true,
            bluetooth: true,
            volume: true,
            brightness: true,
            media: true,
            notifications: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealthPayload {
    pub name: String,
    pub version: String,
    pub state: String,
    pub last_error: Option<String>,
    pub error_count: u32,
    pub uptime_secs: u64,
    pub memory_usage: Option<u64>,
    pub disabled: bool,
    pub is_core: bool,
    pub permissions: Vec<String>,
    pub ui_components: Vec<String>,
    pub config_schema: Option<serde_json::Value>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginListPayload {
    pub plugins: Vec<PluginHealthPayload>,
}
