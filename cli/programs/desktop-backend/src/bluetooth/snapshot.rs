use anyhow::{Context, Result};
use rustc_hash::FxHashMap;
use serde::Serialize;
use zbus::{
    Connection,
    names::OwnedInterfaceName,
    zvariant::{OwnedObjectPath, OwnedValue},
};

use super::protocol::Event;

pub const ADAPTER: &str = "org.bluez.Adapter1";
pub const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";

type Properties = FxHashMap<String, OwnedValue>;
type ManagedObjects = FxHashMap<OwnedObjectPath, FxHashMap<OwnedInterfaceName, Properties>>;

#[zbus::proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.bluez",
    default_path = "/"
)]
trait ObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Adapter {
    path: OwnedObjectPath,
    name: String,
    address: String,
    powered: bool,
    discovering: bool,
    pairable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    path: OwnedObjectPath,
    adapter: OwnedObjectPath,
    address: String,
    name: String,
    icon: String,
    connected: bool,
    paired: bool,
    bonded: bool,
    trusted: bool,
    blocked: bool,
    uuids: Vec<String>,
    rssi: Option<i16>,
    battery: Option<u8>,
    services_resolved: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    adapters: Vec<Adapter>,
    devices: Vec<Device>,
}

impl Snapshot {
    pub async fn read(connection: &Connection) -> Result<Self> {
        let manager = ObjectManagerProxy::new(connection).await?;
        let objects = manager.get_managed_objects().await?;
        let mut adapters = Vec::new();
        let mut devices = Vec::new();
        for (path, interfaces) in objects {
            if let Some(properties) = interfaces.get(ADAPTER) {
                adapters.push(Adapter {
                    path: path.clone(),
                    name: required(properties, "Alias")?,
                    address: required(properties, "Address")?,
                    powered: required(properties, "Powered")?,
                    discovering: required(properties, "Discovering")?,
                    pairable: required(properties, "Pairable")?,
                });
            }
            if let Some(properties) = interfaces.get(DEVICE) {
                let battery = interfaces
                    .get(BATTERY)
                    .map(|properties| optional(properties, "Percentage"))
                    .transpose()?
                    .flatten();
                devices.push(Device {
                    path,
                    adapter: required(properties, "Adapter")?,
                    address: required(properties, "Address")?,
                    name: required(properties, "Alias")?,
                    icon: optional(properties, "Icon")?.unwrap_or_default(),
                    connected: required(properties, "Connected")?,
                    paired: required(properties, "Paired")?,
                    bonded: required(properties, "Bonded")?,
                    trusted: required(properties, "Trusted")?,
                    blocked: required(properties, "Blocked")?,
                    uuids: optional(properties, "UUIDs")?.unwrap_or_default(),
                    rssi: optional(properties, "RSSI")?,
                    battery,
                    services_resolved: required(properties, "ServicesResolved")?,
                });
            }
        }
        adapters.sort_by(|left, right| left.path.cmp(&right.path));
        devices.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self { adapters, devices })
    }

    pub fn into_event(self) -> Event {
        Event::State {
            adapters: self.adapters,
            devices: self.devices,
        }
    }
}

fn required<T>(properties: &Properties, name: &str) -> Result<T>
where
    T: TryFrom<OwnedValue>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    optional(properties, name)?.with_context(|| format!("BlueZ property {name} is missing"))
}

fn optional<T>(properties: &Properties, name: &str) -> Result<Option<T>>
where
    T: TryFrom<OwnedValue>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    properties
        .get(name)
        .map(|value| {
            T::try_from(value.try_clone()?).with_context(|| format!("read BlueZ property {name}"))
        })
        .transpose()
}
