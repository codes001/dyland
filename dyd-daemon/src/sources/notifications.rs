use dyd_types::{IpcEnvelope, IpcEvent, NotificationPayload};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use zbus::{Connection, MessageStream};

use super::calls::{notification_is_call, CallHub, NotifyFields};

type NotifyArgs<'a> = (
    String,
    u32,
    String,
    String,
    String,
    Vec<String>,
    std::collections::HashMap<String, zvariant::Value<'a>>,
    i32,
);

pub struct NotificationSource {
    dbus_conn: Option<Connection>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
    call_hub: Option<Arc<CallHub>>,
}

impl Default for NotificationSource {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationSource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            dbus_conn: None,
            event_broadcast,
            call_hub: None,
        }
    }

    pub fn set_call_hub(&mut self, hub: Arc<CallHub>) {
        self.call_hub = Some(hub);
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

        let stream_conn = match open_monitor().await {
            Some(monitor) => {
                tracing::info!("notification source: monitoring call and notification messages");
                monitor
            }
            None => {
                tracing::warn!("notification source: BecomeMonitor unavailable, using eavesdrop");
                if !register_eavesdrop(&conn).await {
                    return;
                }
                conn
            }
        };

        let mut stream = MessageStream::from(&stream_conn);

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
                            stream = MessageStream::from(&stream_conn);
                        }
                    }
                }
            }
        }
    }

    async fn handle_message(&self, msg: zbus::Message) {
        let hdr = msg.header();
        let interface = match hdr.interface() {
            Some(name) => name.as_str().to_string(),
            None => return,
        };
        let member = match hdr.member() {
            Some(name) => name.as_str().to_string(),
            None => return,
        };

        match (interface.as_str(), member.as_str()) {
            ("org.freedesktop.Notifications", "Notify") => self.handle_notify(msg).await,
            ("org.freedesktop.Notifications", "NotificationClosed") => self.handle_closed(msg),
            ("org.freedesktop.portal.Notification", "AddNotification")
            | ("org.freedesktop.impl.portal.Notification", "AddNotification") => {
                self.handle_portal_add(msg);
            }
            ("org.freedesktop.portal.Notification", "RemoveNotification")
            | ("org.freedesktop.impl.portal.Notification", "RemoveNotification") => {
                self.handle_portal_remove(msg);
            }
            ("org.gtk.Notifications", "AddNotification") => self.handle_gtk_add(msg),
            ("org.gtk.Notifications", "RemoveNotification") => self.handle_gtk_remove(msg),
            _ => {}
        }
    }

    async fn handle_notify(&self, msg: zbus::Message) {
        let body = msg.body();

        let args: Result<NotifyArgs<'_>, _> = body.deserialize();

        let (app_name, replaces_id, app_icon, summary, body_text, actions, hints, _expire_timeout) =
            match args {
                Ok(a) => a,
                Err(e) => {
                    tracing::debug!(error = %e, "notification source: failed to deserialize Notify args");
                    return;
                }
            };

        let action_items: Vec<dyd_types::NotificationActionItem> = actions
            .chunks(2)
            .filter(|chunk| chunk.len() == 2)
            .map(|chunk| dyd_types::NotificationActionItem {
                key: chunk[0].clone(),
                label: chunk[1].clone(),
            })
            .collect();
        let category = hint_string(&hints, "category");
        let action_blob = action_items
            .iter()
            .map(|item| format!("{} {}", item.key, item.label))
            .collect::<Vec<_>>()
            .join(" ");
        let is_call = notification_is_call(&NotifyFields {
            app_name: &app_name,
            summary: &summary,
            body: &body_text,
            category: &category,
            actions: &action_blob,
        });
        let urgency = if is_call {
            "call".to_string()
        } else {
            extract_urgency(&hints)
        };
        if is_call {
            if let Some(hub) = &self.call_hub {
                let key = if replaces_id == 0 {
                    format!("fdo:{app_name}:{summary}")
                } else {
                    format!("fdo:{replaces_id}")
                };
                hub.raise_notification(
                    key,
                    &app_name,
                    &summary,
                    (!body_text.is_empty()).then(|| body_text.clone()),
                );
            }
        }

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

    fn handle_closed(&self, msg: zbus::Message) {
        let Ok((id, _reason)) = msg.body().deserialize::<(u32, u32)>() else {
            return;
        };
        if let Some(hub) = &self.call_hub {
            hub.clear_notification(&format!("fdo:{id}"));
        }
    }

    fn handle_portal_add(&self, msg: zbus::Message) {
        let body = msg.body();
        let Ok((id, fields)) = body
            .deserialize::<(String, std::collections::HashMap<String, zvariant::Value>)>()
        else {
            return;
        };
        self.raise_dict_call(&format!("portal:{id}"), &fields);
    }

    fn handle_portal_remove(&self, msg: zbus::Message) {
        let Ok((id,)) = msg.body().deserialize::<(String,)>() else {
            return;
        };
        if let Some(hub) = &self.call_hub {
            hub.clear_notification(&format!("portal:{id}"));
        }
    }

    fn handle_gtk_add(&self, msg: zbus::Message) {
        let body = msg.body();
        let Ok((app_id, id, fields)) = body.deserialize::<(
            String,
            String,
            std::collections::HashMap<String, zvariant::Value>,
        )>() else {
            return;
        };
        let mut fields = fields;
        fields
            .entry("app-name".to_string())
            .or_insert_with(|| zvariant::Value::new(app_id));
        self.raise_dict_call(&format!("gtk:{id}"), &fields);
    }

    fn handle_gtk_remove(&self, msg: zbus::Message) {
        let Ok((_app_id, id)) = msg.body().deserialize::<(String, String)>() else {
            return;
        };
        if let Some(hub) = &self.call_hub {
            hub.clear_notification(&format!("gtk:{id}"));
        }
    }

    fn raise_dict_call(
        &self,
        key: &str,
        fields: &std::collections::HashMap<String, zvariant::Value>,
    ) {
        let summary = hint_string(fields, "title");
        let body = hint_string(fields, "body");
        let app_name = {
            let named = hint_string(fields, "app-name");
            if named.is_empty() {
                String::new()
            } else {
                named
            }
        };
        let category = hint_string(fields, "category");
        let priority = hint_string(fields, "priority");
        if !notification_is_call(&NotifyFields {
            app_name: &app_name,
            summary: &summary,
            body: &body,
            category: &category,
            actions: &priority,
        }) {
            return;
        }
        let shown_app = if app_name.is_empty() {
            summary.clone()
        } else {
            app_name
        };
        if let Some(hub) = &self.call_hub {
            hub.raise_notification(key.to_string(), &shown_app, &summary, Some(body.clone()));
        }
        let payload = NotificationPayload {
            id: 0,
            app_name: shown_app,
            app_icon: None,
            summary,
            body: (!body.is_empty()).then_some(body),
            urgency: "call".to_string(),
            actions: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        };
        let _ = self
            .event_broadcast
            .send(IpcEnvelope::event(IpcEvent::NotificationReceived(payload)));
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

fn hint_string(
    hints: &std::collections::HashMap<String, zvariant::Value<'_>>,
    key: &str,
) -> String {
    let Some(value) = hints.get(key) else {
        return String::new();
    };
    match value {
        zvariant::Value::Str(text) => text.to_string(),
        zvariant::Value::Value(inner) => match inner.as_ref() {
            zvariant::Value::Str(text) => text.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

async fn open_monitor() -> Option<Connection> {
    let conn = Connection::session().await.ok()?;
    let proxy = zbus::fdo::MonitoringProxy::new(&conn).await.ok()?;
    let rules = [
        "type='method_call',interface='org.freedesktop.Notifications',member='Notify'",
        "type='signal',interface='org.freedesktop.Notifications',member='NotificationClosed'",
        "type='method_call',interface='org.freedesktop.portal.Notification',member='AddNotification'",
        "type='method_call',interface='org.freedesktop.portal.Notification',member='RemoveNotification'",
        "type='method_call',interface='org.freedesktop.impl.portal.Notification',member='AddNotification'",
        "type='method_call',interface='org.freedesktop.impl.portal.Notification',member='RemoveNotification'",
        "type='method_call',interface='org.gtk.Notifications',member='AddNotification'",
        "type='method_call',interface='org.gtk.Notifications',member='RemoveNotification'",
    ];
    let mut parsed = Vec::with_capacity(rules.len());
    for rule in rules {
        parsed.push(zbus::MatchRule::try_from(rule).ok()?);
    }
    proxy.become_monitor(&parsed, 0).await.ok()?;
    Some(conn)
}

async fn register_eavesdrop(conn: &Connection) -> bool {
    let rules = [
        "interface='org.freedesktop.Notifications',member='Notify',type='method_call',eavesdrop='true'",
        "interface='org.freedesktop.Notifications',member='NotificationClosed',type='signal',eavesdrop='true'",
        "interface='org.freedesktop.portal.Notification',member='AddNotification',type='method_call',eavesdrop='true'",
        "interface='org.gtk.Notifications',member='AddNotification',type='method_call',eavesdrop='true'",
    ];
    for rule in rules {
        if let Err(err) = conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "AddMatch",
                &(rule),
            )
            .await
        {
            tracing::warn!(error = %err, "notification source: failed to register match rule");
            return false;
        }
    }
    true
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
