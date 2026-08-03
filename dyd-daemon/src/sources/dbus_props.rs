use std::convert::TryFrom;

use zbus::Connection;
use zbus::zvariant::OwnedValue;

pub async fn get_property<T>(
    conn: &Connection,
    service: &str,
    path: &str,
    interface: &str,
    property: &str,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: TryFrom<OwnedValue>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let msg = conn
        .call_method(
            Some(service),
            path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(interface, property),
        )
        .await?;

    let value: OwnedValue = msg.body().deserialize()?;
    Ok(T::try_from(value)?)
}

pub async fn get_property_opt<T>(
    conn: &Connection,
    service: &str,
    path: &str,
    interface: &str,
    property: &str,
) -> Option<T>
where
    T: TryFrom<OwnedValue>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    get_property(conn, service, path, interface, property)
        .await
        .ok()
}
