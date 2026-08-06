use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, Duration};
use zbus::Connection;

use dyd_types::{IpcEnvelope, IpcEvent, WifiPayload};

use super::dbus_props::get_property;

const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const NM_IFACE: &str = "org.freedesktop.NetworkManager";
const CONN_ACTIVE_IFACE: &str = "org.freedesktop.NetworkManager.Connection.Active";
const AP_IFACE: &str = "org.freedesktop.NetworkManager.AccessPoint";
const IP4_IFACE: &str = "org.freedesktop.NetworkManager.IP4Config";

const NM_ACTIVE_CONN_STATE_ACTIVATED: u32 = 2;

const NM_AP_SEC_PAIR_WEP40: u32 = 0x0000_0001;
const NM_AP_SEC_PAIR_WEP104: u32 = 0x0000_0002;
const NM_AP_SEC_GROUP_WEP40: u32 = 0x0000_0020;
const NM_AP_SEC_GROUP_WEP104: u32 = 0x0000_0040;
const NM_AP_SEC_KEY_MGMT_PSK: u32 = 0x0000_0200;
const NM_AP_SEC_KEY_MGMT_802_1X: u32 = 0x0000_0400;
const NM_AP_SEC_KEY_MGMT_SAE: u32 = 0x0000_0800;
const NM_AP_SEC_KEY_MGMT_OWE: u32 = 0x0000_1000;

const NM_AP_PATH_PREFIX: &str = "/org/freedesktop/NetworkManager/AccessPoint";

#[derive(Debug, Clone, PartialEq)]
struct WifiState {
    ssid: Option<String>,
    strength: u8,
    state: String,
    ip_address: Option<String>,
    frequency_mhz: Option<u32>,
    security: Option<String>,
}

impl Default for WifiState {
    fn default() -> Self {
        Self {
            ssid: None,
            strength: 0,
            state: "none".to_string(),
            ip_address: None,
            frequency_mhz: None,
            security: None,
        }
    }
}

pub struct WifiSource {
    dbus_conn: Option<Connection>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
    last: Arc<tokio::sync::RwLock<WifiState>>,
}

impl WifiSource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            dbus_conn: None,
            event_broadcast,
            last: Arc::new(tokio::sync::RwLock::new(WifiState::default())),
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
                    tracing::info!("wifi source shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = self.poll().await {
                        tracing::debug!(error = %e, "wifi poll cycle error");
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

        let state = self.fetch_wifi_state(conn).await?;
        self.emit_if_changed(state).await;
        Ok(())
    }

    async fn fetch_wifi_state(
        &self,
        conn: &Connection,
    ) -> Result<WifiState, Box<dyn std::error::Error + Send + Sync>> {
        let primary_path = get_property::<zvariant::OwnedObjectPath>(conn, NM_SERVICE, NM_PATH, NM_IFACE, "PrimaryConnection")
            .await
            .map(|p| p.to_string())
            .unwrap_or_else(|e| {
                tracing::debug!(error = %e, "wifi source: NetworkManager PrimaryConnection unavailable");
                String::new()
            });

        if primary_path.is_empty() || primary_path == "/" {
            return Ok(WifiState {
                state: "none".to_string(),
                ..Default::default()
            });
        }

        let specific = get_property::<zvariant::OwnedObjectPath>(
            conn,
            NM_SERVICE,
            &primary_path,
            CONN_ACTIVE_IFACE,
            "SpecificObject",
        )
        .await
        .map(|p| p.to_string())
        .unwrap_or_default();

        if specific.is_empty() || specific == "/" {
            return Ok(WifiState {
                state: "disconnected".to_string(),
                ..Default::default()
            });
        }

        if !specific.starts_with(NM_AP_PATH_PREFIX) {
            return Ok(WifiState {
                state: "disconnected".to_string(),
                ..Default::default()
            });
        }

        let conn_state =
            get_property::<u32>(conn, NM_SERVICE, &primary_path, CONN_ACTIVE_IFACE, "State")
                .await
                .unwrap_or(0);

        let ap_path = specific;

        let strength = get_property::<u8>(conn, NM_SERVICE, &ap_path, AP_IFACE, "Strength")
            .await
            .unwrap_or(0);

        let ssid = get_property::<Vec<u8>>(conn, NM_SERVICE, &ap_path, AP_IFACE, "Ssid")
            .await
            .ok()
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string());

        let frequency_mhz = get_property::<u32>(conn, NM_SERVICE, &ap_path, AP_IFACE, "Frequency")
            .await
            .ok()
            .filter(|f| *f > 0);

        let security = self.fetch_security(conn, &ap_path).await;

        let ip_address = self.fetch_ip(conn, &primary_path).await;

        let state = if conn_state == NM_ACTIVE_CONN_STATE_ACTIVATED {
            "connected".to_string()
        } else if ssid.is_some() {
            "connecting".to_string()
        } else {
            "disconnected".to_string()
        };

        Ok(WifiState {
            ssid,
            strength,
            state,
            ip_address,
            frequency_mhz,
            security,
        })
    }

    async fn fetch_security(&self, conn: &Connection, ap_path: &str) -> Option<String> {
        let wpa_flags = get_property::<u32>(conn, NM_SERVICE, ap_path, AP_IFACE, "WpaFlags")
            .await
            .ok()?;

        let rsn_flags = get_property::<u32>(conn, NM_SERVICE, ap_path, AP_IFACE, "RsnFlags")
            .await
            .ok()?;

        let flags = wpa_flags | rsn_flags;
        if flags == 0 {
            return Some("Open".to_string());
        }
        if flags & (NM_AP_SEC_KEY_MGMT_SAE | NM_AP_SEC_KEY_MGMT_OWE) != 0 {
            return Some("WPA3".to_string());
        }
        if flags & (NM_AP_SEC_KEY_MGMT_PSK | NM_AP_SEC_KEY_MGMT_802_1X) != 0 {
            return Some("WPA2".to_string());
        }
        if flags
            & (NM_AP_SEC_PAIR_WEP40
                | NM_AP_SEC_PAIR_WEP104
                | NM_AP_SEC_GROUP_WEP40
                | NM_AP_SEC_GROUP_WEP104)
            != 0
        {
            return Some("WEP".to_string());
        }
        Some("WPA".to_string())
    }

    async fn fetch_ip(&self, conn: &Connection, active_conn_path: &str) -> Option<String> {
        let ip4_path = get_property::<zvariant::OwnedObjectPath>(
            conn,
            NM_SERVICE,
            active_conn_path,
            CONN_ACTIVE_IFACE,
            "Ip4Config",
        )
        .await
        .ok()?
        .to_string();

        if ip4_path.is_empty() || ip4_path == "/" {
            return None;
        }

        let addresses =
            get_property::<Vec<Vec<u32>>>(conn, NM_SERVICE, &ip4_path, IP4_IFACE, "Addresses")
                .await
                .ok()?;

        addresses
            .first()
            .and_then(|addr| addr.first())
            .map(|ip| format_ipv4(*ip))
    }

    async fn emit_if_changed(&self, state: WifiState) {
        let mut last = self.last.write().await;
        if *last == state {
            return;
        }
        *last = state.clone();

        let payload = WifiPayload {
            ssid: state.ssid.clone(),
            strength: state.strength,
            state: state.state.clone(),
            ip_address: state.ip_address.clone(),
            frequency_mhz: state.frequency_mhz,
            security: state.security.clone(),
        };

        tracing::debug!(
            ssid = %payload.ssid.as_deref().unwrap_or("-"),
            strength = %payload.strength,
            state = %payload.state,
            "wifi changed"
        );

        let envelope = IpcEnvelope::event(IpcEvent::WifiChanged(payload));
        let _ = self.event_broadcast.send(envelope);
    }
}

impl Default for WifiSource {
    fn default() -> Self {
        Self::new()
    }
}

fn format_ipv4(addr: u32) -> String {
    let bytes = addr.to_be_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ipv4() {
        assert_eq!(format_ipv4(0x7F000001), "127.0.0.1");
        assert_eq!(format_ipv4(0x0A000001), "10.0.0.1");
        assert_eq!(format_ipv4(0xC0A80001), "192.168.0.1");
    }

    #[test]
    fn test_wifi_state_change_detection() {
        let a = WifiState {
            ssid: Some("HomeWiFi".to_string()),
            strength: 80,
            state: "connected".to_string(),
            ip_address: Some("192.168.1.5".to_string()),
            frequency_mhz: Some(5200),
            security: Some("WPA2".to_string()),
        };
        let b = a.clone();
        let c = WifiState {
            strength: 40,
            ..a.clone()
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
