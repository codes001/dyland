use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::ConfigStore;
use crate::plugins::PluginManager;
use crate::sources::battery::BatterySource;
use crate::sources::bluetooth::BluetoothSource;
use crate::sources::brightness::BrightnessSource;
use crate::sources::media::MediaSource;
use crate::sources::notifications::NotificationSource;
use crate::sources::volume::VolumeSource;
use crate::sources::wifi::WifiSource;
use dyd_types::IpcEnvelope;

#[derive(Clone)]
pub struct SourceContext {
    pub media_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub notification_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub battery_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub wifi_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub bluetooth_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub volume_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub brightness_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub media_source: Option<Arc<MediaSource>>,
    pub notification_source: Option<Arc<NotificationSource>>,
    pub battery_source: Option<Arc<BatterySource>>,
    pub wifi_source: Option<Arc<WifiSource>>,
    pub bluetooth_source: Option<Arc<BluetoothSource>>,
    pub volume_source: Option<Arc<VolumeSource>>,
    pub brightness_source: Option<Arc<BrightnessSource>>,
    pub plugin_manager: Option<Arc<PluginManager>>,
    pub plugin_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
    pub config_store: Option<Arc<ConfigStore>>,
    /// Relays `ConfigChanged` events (wrapped as `IpcEnvelope`) to every open
    /// connection when any client updates the config.
    pub config_broadcast: Option<broadcast::Sender<IpcEnvelope>>,
}

impl SourceContext {
    pub fn broadcast_receivers(&self) -> Vec<broadcast::Receiver<IpcEnvelope>> {
        let mut receivers = Vec::new();
        tracing::debug!(
            media = self.media_broadcast.is_some(),
            config = self.config_broadcast.is_some(),
            plugin = self.plugin_broadcast.is_some(),
            "building broadcast receivers"
        );
        if let Some(tx) = &self.media_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.notification_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.battery_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.wifi_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.bluetooth_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.volume_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.brightness_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.plugin_broadcast {
            receivers.push(tx.subscribe());
        }
        if let Some(tx) = &self.config_broadcast {
            receivers.push(tx.subscribe());
        }
        receivers
    }
}
