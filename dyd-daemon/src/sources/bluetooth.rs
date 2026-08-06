use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, Duration};
use zbus::Connection;

use dyd_types::{BluetoothDevice, BluetoothPayload, IpcEnvelope, IpcEvent};

const BLUEZ_SERVICE: &str = "org.bluez";
const ADAPTER_PREFIX: &str = "/org/bluez/hci";
const DEVICE_PREFIX: &str = "/dev_";

#[derive(Debug, Clone, PartialEq, Default)]
struct BluetoothState {
    powered: bool,
    discoverable: bool,
    devices: Vec<BluetoothDevice>,
    present: bool,
}

pub struct BluetoothSource {
    dbus_conn: Option<Connection>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
    last: Arc<tokio::sync::RwLock<BluetoothState>>,
}

impl BluetoothSource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            dbus_conn: None,
            event_broadcast,
            last: Arc::new(tokio::sync::RwLock::new(BluetoothState::default())),
        }
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.event_broadcast.clone()
    }

    pub fn set_dbus_connection(&mut self, conn: Connection) {
        self.dbus_conn = Some(conn);
    }

    pub async fn start_polling(self: Arc<Self>, mut shutdown_rx: watch::Receiver<()>) {
        let mut tick = interval(Duration::from_millis(500));
        tick.tick().await;

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    tracing::info!("bluetooth source shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = self.poll().await {
                        tracing::debug!(error = %e, "bluetooth poll cycle error");
                    }
                }
            }
        }
    }

    async fn poll(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Ok(()),
        };

        let state = self.fetch_bluetooth_state(conn).await?;
        self.emit_if_changed(state).await;
        Ok(())
    }

    async fn fetch_bluetooth_state(
        &self,
        conn: &Connection,
    ) -> Result<BluetoothState, Box<dyn std::error::Error + Send + Sync>> {
        let msg = conn
            .call_method(
                Some(BLUEZ_SERVICE),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await?;
        let body = msg.body();
        let objects: HashMap<
            zvariant::OwnedObjectPath,
            HashMap<String, HashMap<String, zvariant::Value>>,
        > = body.deserialize()?;

        if objects.is_empty() {
            return Ok(BluetoothState::default());
        }

        let mut powered = false;
        let mut discoverable = false;
        let mut present = false;
        let mut devices = Vec::new();

        for (path, interfaces) in &objects {
            if path.starts_with(ADAPTER_PREFIX) {
                present = true;
                if let Some(props) = interfaces.get("org.bluez.Adapter1") {
                    if let Some(val) = props.get("Powered") {
                        if let Ok(b) = val.downcast_ref::<bool>() {
                            powered = b;
                        }
                    }
                    if let Some(val) = props.get("Discoverable") {
                        if let Ok(b) = val.downcast_ref::<bool>() {
                            discoverable = b;
                        }
                    }
                }
            }
            if let Some(props) = interfaces.get("org.bluez.Device1") {
                if !path.contains(DEVICE_PREFIX) {
                    continue;
                }
                let connected = props
                    .get("Connected")
                    .and_then(|v| v.downcast_ref::<bool>().ok())
                    .unwrap_or(false);

                let name = props
                    .get("Name")
                    .and_then(|v| v.downcast_ref::<String>().ok())
                    .unwrap_or_else(|| "Unknown Device".to_string());

                let icon = props
                    .get("Icon")
                    .and_then(|v| v.downcast_ref::<String>().ok());

                let address = props
                    .get("Address")
                    .and_then(|v| v.downcast_ref::<String>().ok())
                    .unwrap_or_else(|| path.to_string());

                devices.push(BluetoothDevice {
                    name,
                    connected,
                    icon,
                    address,
                });
            }
        }

        devices.sort_by(|a, b| a.address.cmp(&b.address));

        Ok(BluetoothState {
            powered,
            discoverable,
            devices,
            present,
        })
    }

    pub async fn set_powered(
        &self,
        powered: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Err("no D-Bus connection".into()),
        };

        let adapter = self.find_adapter(conn).await?;
        conn.call_method(
            Some(BLUEZ_SERVICE),
            adapter.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &(
                "org.bluez.Adapter1",
                "Powered",
                zvariant::Value::Bool(powered),
            ),
        )
        .await?;

        tracing::debug!(powered = %powered, "bluetooth powered set");
        Ok(())
    }

    async fn find_adapter(
        &self,
        conn: &Connection,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let msg = conn
            .call_method(
                Some(BLUEZ_SERVICE),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await?;
        let body = msg.body();
        let objects: HashMap<
            zvariant::OwnedObjectPath,
            HashMap<String, HashMap<String, zvariant::Value>>,
        > = body.deserialize()?;

        let mut adapters: Vec<String> = objects
            .keys()
            .filter(|p| p.starts_with(ADAPTER_PREFIX))
            .map(|p| p.to_string())
            .collect();
        adapters.sort();

        adapters
            .first()
            .cloned()
            .ok_or_else(|| "no bluetooth adapter found".into())
    }

    async fn emit_if_changed(&self, state: BluetoothState) {
        let mut last = self.last.write().await;
        if *last == state {
            return;
        }
        *last = state.clone();

        let connected_count = state.devices.iter().filter(|d| d.connected).count();

        tracing::debug!(
            powered = %state.powered,
            discoverable = %state.discoverable,
            connected = %connected_count,
            present = %state.present,
            "bluetooth changed"
        );

        let payload = BluetoothPayload {
            powered: state.powered,
            discoverable: state.discoverable,
            present: state.present,
            devices: state.devices,
        };

        let envelope = IpcEnvelope::event(IpcEvent::BluetoothChanged(payload));
        let _ = self.event_broadcast.send(envelope);
    }
}

impl Default for BluetoothSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_not_present() {
        let s = BluetoothState::default();
        assert!(!s.present);
        assert!(!s.powered);
        assert!(s.devices.is_empty());
    }

    #[test]
    fn test_bluetooth_state_change_detection() {
        let a = BluetoothState {
            powered: true,
            discoverable: false,
            devices: vec![BluetoothDevice {
                name: "Headset".to_string(),
                connected: true,
                icon: Some("audio-headset".to_string()),
                address: "AA:BB:CC:DD:EE:FF".to_string(),
            }],
            present: true,
        };
        let b = a.clone();
        let c = BluetoothState {
            powered: false,
            ..a.clone()
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
