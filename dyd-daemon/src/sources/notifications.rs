use dyd_types::{IpcEnvelope, IpcEvent, NotificationPayload};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use zbus::{Connection, MessageStream};

pub struct NotificationSource {
    dbus_conn: Option<Connection>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
}

impl NotificationSource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            dbus_conn: None,
            event_broadcast,
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
                tracing::warn!("notification source: no D-Bus connection available");
                return;
            }
        };

        let match_rule =
            "interface='org.freedesktop.Notifications',member='Notify',type='method_call',eavesdrop='true'";

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
            Ok(_) => tracing::info!("notification source: registered D-Bus match rule for Notify"),
            Err(e) => {
                tracing::warn!(error = %e, "notification source: failed to register match rule");
                return;
            }
        }

        let mut stream = MessageStream::from(&conn);

        tracing::info!("notification source: started listening for Notify calls");

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    tracing::info!("notification source shutting down");
                    break;
                }
                msg = stream.next() => {
                    match msg {
                        Some(Ok(msg)) => self.handle_message(msg).await,
                        Some(Err(e)) => {
                            tracing::debug!(error = %e, "notification source: message stream error");
                        }
                        None => {
                            tracing::warn!("notification source: message stream ended, will retry");
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

    async fn handle_message(&self, msg: zbus::Message) {
        let hdr = msg.header();
        let interface = match hdr.interface() {
            Some(i) => i,
            None => return,
        };
        let member = match hdr.member() {
            Some(m) => m,
            None => return,
        };

        if interface.as_str() != "org.freedesktop.Notifications" || member.as_str() != "Notify" {
            return;
        }

        let body = msg.body();

        let args: Result<
            (
                String,
                u32,
                String,
                String,
                String,
                Vec<String>,
                std::collections::HashMap<String, zvariant::Value<'_>>,
                i32,
            ),
            _,
        > = body.deserialize();

        let (app_name, replaces_id, app_icon, summary, body_text, actions, hints, _expire_timeout) =
            match args {
                Ok(a) => a,
                Err(e) => {
                    tracing::debug!(error = %e, "notification source: failed to deserialize Notify args");
                    return;
                }
            };

        let urgency = extract_urgency(&hints);

        let action_items: Vec<dyd_types::NotificationActionItem> = actions
            .chunks(2)
            .filter(|chunk| chunk.len() == 2)
            .map(|chunk| dyd_types::NotificationActionItem {
                key: chunk[0].clone(),
                label: chunk[1].clone(),
            })
            .collect();

        let payload = NotificationPayload {
            id: replaces_id,
            app_name,
            app_icon: (!app_icon.is_empty()).then_some(app_icon),
            summary,
            body: (!body_text.is_empty()).then_some(body_text),
            urgency,
            actions: action_items,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        };

        tracing::debug!(
            app = %payload.app_name,
            summary = %payload.summary,
            urgency = %payload.urgency,
            "notification received"
        );

        let envelope = IpcEnvelope::event(IpcEvent::NotificationReceived(payload));
        let _ = self.event_broadcast.send(envelope);
    }

    pub async fn invoke_action(
        &self,
        id: u32,
        action_key: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Err("no D-Bus connection".into()),
        };
        match conn
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "InvokeAction",
                &(id, action_key),
            )
            .await
        {
            Ok(_) => tracing::debug!(id, action_key = %action_key, "notification action invoked"),
            Err(e) => {
                tracing::debug!(error = %e, id, action_key = %action_key, "InvokeAction not supported")
            }
        }
        Ok(())
    }

    pub async fn close_notification(&self, id: u32) -> Result<(), Box<dyn std::error::Error>> {
        let conn = match &self.dbus_conn {
            Some(c) => c,
            None => return Err("no D-Bus connection".into()),
        };
        conn.call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "CloseNotification",
            &(id),
        )
        .await?;
        tracing::debug!(id, "notification closed via D-Bus");
        Ok(())
    }
}

fn extract_urgency(hints: &std::collections::HashMap<String, zvariant::Value<'_>>) -> String {
    if let Some(val) = hints.get("urgency") {
        if let Ok(byte) = val.downcast_ref::<u8>() {
            return match byte {
                0 => "low".to_string(),
                1 => "normal".to_string(),
                2 => "critical".to_string(),
                _ => "normal".to_string(),
            };
        }
        if let Ok(val) = val.downcast_ref::<i32>() {
            return match val {
                0 => "low".to_string(),
                1 => "normal".to_string(),
                2 => "critical".to_string(),
                _ => "normal".to_string(),
            };
        }
    }
    "normal".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_extract_urgency_default() {
        let hints = HashMap::new();
        assert_eq!(extract_urgency(&hints), "normal");
    }

    #[test]
    fn test_extract_urgency_u8_low() {
        let mut hints = HashMap::new();
        hints.insert("urgency".to_string(), zvariant::Value::U8(0));
        assert_eq!(extract_urgency(&hints), "low");
    }

    #[test]
    fn test_extract_urgency_u8_normal() {
        let mut hints = HashMap::new();
        hints.insert("urgency".to_string(), zvariant::Value::U8(1));
        assert_eq!(extract_urgency(&hints), "normal");
    }

    #[test]
    fn test_extract_urgency_u8_critical() {
        let mut hints = HashMap::new();
        hints.insert("urgency".to_string(), zvariant::Value::U8(2));
        assert_eq!(extract_urgency(&hints), "critical");
    }

    #[test]
    fn test_extract_urgency_i32() {
        let mut hints = HashMap::new();
        hints.insert("urgency".to_string(), zvariant::Value::I32(2));
        assert_eq!(extract_urgency(&hints), "critical");
    }

    #[test]
    fn test_extract_urgency_no_urgency_key() {
        let mut hints = HashMap::new();
        hints.insert("other".to_string(), zvariant::Value::U8(0));
        assert_eq!(extract_urgency(&hints), "normal");
    }
}
