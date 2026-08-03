use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, Duration};
use zbus::Connection;

use dyd_types::{BrightnessPayload, IpcEnvelope, IpcEvent};

const BACKLIGHT_DIR: &str = "/sys/class/backlight";
const GNOME_POWER_SERVICE: &str = "org.gnome.SettingsDaemon.Power";
const GNOME_POWER_PATH: &str = "/org/gnome/SettingsDaemon/Power";
const GNOME_SCREEN_IFACE: &str = "org.gnome.SettingsDaemon.Power.Screen";

#[derive(Debug, Clone, PartialEq, Default)]
struct BrightnessState {
    brightness: u8,
    has_backlight: bool,
}

pub struct BrightnessSource {
    dbus_conn: Option<Connection>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
    last: Arc<tokio::sync::RwLock<BrightnessState>>,
}

impl BrightnessSource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            dbus_conn: None,
            event_broadcast,
            last: Arc::new(tokio::sync::RwLock::new(BrightnessState::default())),
        }
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.event_broadcast.clone()
    }

    pub fn set_dbus_connection(&mut self, conn: Connection) {
        self.dbus_conn = Some(conn);
    }

    pub async fn start_polling(self: Arc<Self>, mut shutdown_rx: watch::Receiver<()>) {
        let mut tick = interval(Duration::from_millis(200));
        tick.tick().await;

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    tracing::info!("brightness source shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = self.poll().await {
                        tracing::debug!(error = %e, "brightness poll cycle error");
                    }
                }
            }
        }
    }

    async fn poll(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = self.fetch_state().await?;
        self.emit_if_changed(state).await;
        Ok(())
    }

    async fn fetch_state(
        &self,
    ) -> Result<BrightnessState, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(device) = find_backlight_device() {
            let max = read_u32(&format!("{device}/max_brightness")).await;
            if max == 0 {
                return Ok(BrightnessState::default());
            }

            let current = read_u32(&format!("{device}/brightness")).await;
            let brightness = ((current.clamp(0, max) as f64 / max as f64) * 100.0).round() as u8;
            return Ok(BrightnessState {
                brightness,
                has_backlight: true,
            });
        }

        match self.fetch_gnome_brightness().await {
            Ok(Some(brightness)) => Ok(BrightnessState {
                brightness,
                has_backlight: true,
            }),
            _ => Ok(BrightnessState::default()),
        }
    }

    async fn fetch_gnome_brightness(
        &self,
    ) -> Result<Option<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Ok(None),
        };

        let result = conn
            .call_method(
                Some(GNOME_POWER_SERVICE),
                GNOME_POWER_PATH,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(GNOME_SCREEN_IFACE, "Brightness"),
            )
            .await;

        let value = match result {
            Ok(msg) => msg.body().deserialize::<i32>()?,
            Err(e) => {
                tracing::debug!(error = %e, "gnome power brightness unavailable");
                return Ok(None);
            }
        };

        let brightness = value.clamp(0, 100) as u8;
        Ok(Some(brightness))
    }

    async fn emit_if_changed(&self, state: BrightnessState) {
        let mut last = self.last.write().await;
        if *last == state {
            return;
        }
        *last = state.clone();

        if !state.has_backlight {
            return;
        }

        tracing::debug!(brightness = %state.brightness, "brightness changed");

        let payload = BrightnessPayload {
            brightness: state.brightness,
            has_backlight: true,
        };

        let envelope = IpcEnvelope::event(IpcEvent::BrightnessChanged(payload));
        let _ = self.event_broadcast.send(envelope);
    }

    pub async fn set_brightness(
        &self,
        brightness: u8,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(device) = find_backlight_device() {
            let max = read_u32(&format!("{device}/max_brightness")).await;
            if max == 0 {
                return Err("backlight max_brightness is zero".into());
            }

            let value = ((brightness.clamp(0, 100) as f64 / 100.0) * max as f64).round() as u32;
            let path = format!("{device}/brightness");
            match std::fs::write(&path, value.to_string()) {
                Ok(()) => {
                    tracing::debug!(brightness = %brightness, value, "brightness set");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(path, error = %e, "sysfs brightness write failed, falling back to D-Bus");
                }
            }
        }

        if let Some(conn) = &self.dbus_conn {
            conn.call_method(
                Some(GNOME_POWER_SERVICE),
                GNOME_POWER_PATH,
                Some(GNOME_SCREEN_IFACE),
                "SetBrightness",
                &(brightness.clamp(0, 100) as i32),
            )
            .await?;
            tracing::debug!(brightness = %brightness, "brightness set via gnome power");
            return Ok(());
        }

        Err("no backlight device found".into())
    }
}

impl Default for BrightnessSource {
    fn default() -> Self {
        Self::new()
    }
}

fn find_backlight_device() -> Option<String> {
    let entries = std::fs::read_dir(BACKLIGHT_DIR).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.join("brightness").exists() && path.join("max_brightness").exists() {
            return path.to_str().map(|p| p.to_string());
        }
    }
    None
}

async fn read_u32(path: &str) -> u32 {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => content.trim().parse::<u32>().unwrap_or(0),
        Err(e) => {
            tracing::debug!(path, error = %e, "failed to read backlight file");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let s = BrightnessState::default();
        assert!(!s.has_backlight);
        assert_eq!(s.brightness, 0);
    }

    #[test]
    fn test_percent_calculation() {
        let brightness = ((50_u32.clamp(0, 100) as f64 / 100.0) * 100.0).round() as u8;
        assert_eq!(brightness, 50);
    }
}
