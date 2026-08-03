use dyd_types::{BatteryPayload, IpcEnvelope, IpcEvent};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use zbus::{Connection, MessageStream};

use super::dbus_props::{get_property, get_property_opt};

const DISPLAY_DEVICE_PATH: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const U_DEVICE_IFACE: &str = "org.freedesktop.UPower.Device";

#[derive(Debug, Clone, PartialEq)]
struct BatteryState {
    percentage: f64,
    state: String,
    time_to_empty: Option<u64>,
    time_to_full: Option<u64>,
    is_present: bool,
    health: Option<f64>,
}

impl Default for BatteryState {
    fn default() -> Self {
        Self {
            percentage: 0.0,
            state: "unknown".to_string(),
            time_to_empty: None,
            time_to_full: None,
            is_present: false,
            health: None,
        }
    }
}

pub struct BatterySource {
    dbus_conn: Option<Connection>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
    last: Arc<tokio::sync::RwLock<BatteryState>>,
}

impl BatterySource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            dbus_conn: None,
            event_broadcast,
            last: Arc::new(tokio::sync::RwLock::new(BatteryState::default())),
        }
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.event_broadcast.clone()
    }

    pub fn set_dbus_connection(&mut self, conn: Connection) {
        self.dbus_conn = Some(conn);
    }

    pub async fn start_listening(self: Arc<Self>, mut shutdown_rx: watch::Receiver<()>) {
        let conn = match &self.dbus_conn {
            Some(c) => c.clone(),
            None => {
                tracing::warn!("battery source: no D-Bus connection available");
                return;
            }
        };

        loop {
            if shutdown_rx.has_changed().unwrap_or(false) {
                tracing::info!("battery source shutting down");
                break;
            }

            match self.initial_fetch(&conn).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!(error = %e, "battery source: initial fetch failed, retrying");
                    if tokio::select! {
                        _ = shutdown_rx.changed() => true,
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => false,
                    } {
                        break;
                    }
                    continue;
                }
            }

            let match_rule = format!(
                "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='{DISPLAY_DEVICE_PATH}'"
            );

            match conn
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "AddMatch",
                    &(match_rule),
                )
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        "battery source: registered D-Bus match rule for UPower DisplayDevice"
                    );
                    break;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "battery source: failed to register match rule, retrying");
                    if tokio::select! {
                        _ = shutdown_rx.changed() => true,
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => false,
                    } {
                        return;
                    }
                }
            }
        }

        let mut stream = MessageStream::from(&conn);

        tracing::info!("battery source: started listening for UPower PropertiesChanged");

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    tracing::info!("battery source shutting down");
                    break;
                }
                msg = stream.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            if let Err(e) = self.handle_message(msg).await {
                                tracing::debug!(error = %e, "battery source: handle message error");
                            }
                        }
                        Some(Err(e)) => {
                            tracing::debug!(error = %e, "battery source: message stream error");
                        }
                        None => {
                            tracing::warn!("battery source: message stream ended, will retry");
                            tokio::select! {
                                _ = shutdown_rx.changed() => break,
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {}
                            }
                            stream = MessageStream::from(&conn);
                        }
                    }
                }
            }
        }
    }

    async fn initial_fetch(
        &self,
        conn: &Connection,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = self.fetch_battery_state(conn).await?;
        self.emit_if_changed(state).await;
        Ok(())
    }

    async fn handle_message(
        &self,
        msg: zbus::Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let hdr = msg.header();
        let path = match hdr.path() {
            Some(p) => p,
            None => return Ok(()),
        };
        let interface = match hdr.interface() {
            Some(i) => i,
            None => return Ok(()),
        };
        let member = match hdr.member() {
            Some(m) => m,
            None => return Ok(()),
        };

        if path.as_str() != DISPLAY_DEVICE_PATH
            || interface.as_str() != "org.freedesktop.DBus.Properties"
            || member.as_str() != "PropertiesChanged"
        {
            return Ok(());
        }

        let conn = match &self.dbus_conn {
            Some(c) => c.clone(),
            None => return Ok(()),
        };

        let state = self.fetch_battery_state(&conn).await?;
        self.emit_if_changed(state).await;
        Ok(())
    }

    async fn fetch_battery_state(
        &self,
        conn: &Connection,
    ) -> Result<BatteryState, Box<dyn std::error::Error + Send + Sync>> {
        let state = get_property::<u32>(conn, "org.freedesktop.UPower", DISPLAY_DEVICE_PATH, U_DEVICE_IFACE, "State")
            .await
            .unwrap_or(0);

        let percentage = match get_property::<f64>(conn, "org.freedesktop.UPower", DISPLAY_DEVICE_PATH, U_DEVICE_IFACE, "Percentage")
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!(error = %e, "battery source: Percentage property unavailable");
                let last = self.last.read().await;
                return Ok(BatteryState {
                    is_present: last.is_present,
                    ..Default::default()
                });
            }
        };

        let is_present = get_property::<bool>(conn, "org.freedesktop.UPower", DISPLAY_DEVICE_PATH, U_DEVICE_IFACE, "IsPresent")
            .await
            .unwrap_or(false);

        let time_to_empty = get_property_opt::<i64>(conn, "org.freedesktop.UPower", DISPLAY_DEVICE_PATH, U_DEVICE_IFACE, "TimeToEmpty")
            .await
            .filter(|v| *v > 0)
            .map(|v| v as u64);

        let time_to_full = get_property_opt::<i64>(conn, "org.freedesktop.UPower", DISPLAY_DEVICE_PATH, U_DEVICE_IFACE, "TimeToFull")
            .await
            .filter(|v| *v > 0)
            .map(|v| v as u64);

        let health = self.fetch_health(conn).await;

        let state = map_state(state);

        Ok(BatteryState {
            percentage,
            state,
            time_to_empty,
            time_to_full,
            is_present,
            health,
        })
    }

    async fn fetch_health(&self, conn: &Connection) -> Option<f64> {
        let energy_full = get_property_opt::<f64>(conn, "org.freedesktop.UPower", DISPLAY_DEVICE_PATH, U_DEVICE_IFACE, "EnergyFull")
            .await;

        let energy_full_design =
            get_property_opt::<f64>(conn, "org.freedesktop.UPower", DISPLAY_DEVICE_PATH, U_DEVICE_IFACE, "EnergyFullDesign")
                .await;

        match (energy_full, energy_full_design) {
            (Some(current), Some(design)) if design > 0.0 => {
                Some((current / design) * 100.0).map(|h| h.clamp(0.0, 100.0))
            }
            _ => None,
        }
    }

    async fn emit_if_changed(&self, state: BatteryState) {
        let mut last = self.last.write().await;
        if *last == state {
            return;
        }
        *last = state.clone();

        let payload = BatteryPayload {
            percentage: state.percentage,
            state: state.state.clone(),
            time_to_empty: state.time_to_empty,
            time_to_full: state.time_to_full,
            is_present: state.is_present,
            health: state.health,
        }
        .sanitize();

        tracing::debug!(
            percentage = %payload.percentage,
            state = %payload.state,
            present = %payload.is_present,
            "battery changed"
        );

        let envelope = IpcEnvelope::event(IpcEvent::BatteryChanged(payload));
        let _ = self.event_broadcast.send(envelope);
    }
}

impl Default for BatterySource {
    fn default() -> Self {
        Self::new()
    }
}

fn map_state(state: u32) -> String {
    match state {
        1 => "charging".to_string(),
        2 => "discharging".to_string(),
        3 => "empty".to_string(),
        4 => "full".to_string(),
        5 => "pending-charge".to_string(),
        6 => "pending-discharge".to_string(),
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_state_values() {
        assert_eq!(map_state(0), "unknown");
        assert_eq!(map_state(1), "charging");
        assert_eq!(map_state(2), "discharging");
        assert_eq!(map_state(3), "empty");
        assert_eq!(map_state(4), "full");
        assert_eq!(map_state(5), "pending-charge");
        assert_eq!(map_state(6), "pending-discharge");
        assert_eq!(map_state(99), "unknown");
    }

    #[test]
    fn test_battery_state_change_detection() {
        let a = BatteryState {
            percentage: 85.0,
            state: "discharging".to_string(),
            time_to_empty: Some(5000),
            time_to_full: None,
            is_present: true,
            health: Some(90.0),
        };
        let b = BatteryState {
            percentage: 85.0,
            state: "discharging".to_string(),
            time_to_empty: Some(5000),
            time_to_full: None,
            is_present: true,
            health: Some(90.0),
        };
        let c = BatteryState {
            percentage: 84.0,
            ..a.clone()
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_default_state_not_present() {
        let d = BatteryState::default();
        assert!(!d.is_present);
        assert_eq!(d.state, "unknown");
    }
}
