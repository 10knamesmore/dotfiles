//! NetworkManager queries for the selected interface and its change notifications.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use rustc_hash::FxHashMap;
use tokio::{sync::watch, task::JoinHandle};
use zbus::fdo;
use zbus::message::Type as MessageType;
use zbus::proxy::{Builder, CacheProperties};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue};
use zbus::{Connection, MatchRule, MessageStream, Proxy};

use super::snapshot::{ConnectionKind, Snapshot};

/// Service name of the NetworkManager daemon. It is also the root of the interfaces whose
/// properties end up in a snapshot.
const SERVICE: &str = "org.freedesktop.NetworkManager";

/// Object path of the NetworkManager manager object.
const MANAGER_PATH: ObjectPath<'static> =
    ObjectPath::from_static_str_unchecked("/org/freedesktop/NetworkManager");

/// Interface of the manager object; NetworkManager names it after the daemon.
const MANAGER_INTERFACE: &str = "org.freedesktop.NetworkManager";
const DEVICE_INTERFACE: &str = "org.freedesktop.NetworkManager.Device";
const WIRELESS_INTERFACE: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const IP4_CONFIG_INTERFACE: &str = "org.freedesktop.NetworkManager.IP4Config";
const ACCESS_POINT_INTERFACE: &str = "org.freedesktop.NetworkManager.AccessPoint";
const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";

/// `NM_DEVICE_TYPE_WIFI`; every other device type is reported as ethernet.
const WIFI_DEVICE_TYPE: u32 = 2;

/// Error NetworkManager returns for an interface it has no device for.
const UNKNOWN_DEVICE_ERROR: &str = "org.freedesktop.NetworkManager.UnknownDevice";

/// Object path NetworkManager uses for an unset path property.
const NO_OBJECT: &str = "/";

/// The consumer drains this queue independently of snapshot queries, coalescing changes in watch.
const CHANGE_QUEUE: usize = 256;

/// Read the snapshot of the connection that owns the default route.
///
/// `interface` is `None` when the kernel has no default IPv4 route.
pub async fn read(connection: &Connection, interface: Option<&str>) -> Result<Snapshot> {
    let Some(interface) = interface else {
        return Ok(Snapshot::disconnected());
    };

    let Some(device_path) = device_path(connection, interface).await? else {
        // The kernel routes through an interface NetworkManager does not manage, so only the
        // interface name is known.
        return Ok(Snapshot::ethernet(interface));
    };

    let device = proxy(connection, device_path.clone().into(), DEVICE_INTERFACE).await?;
    let wireless = device.get_property::<u32>("DeviceType").await? == WIFI_DEVICE_TYPE;

    let ip4_config: OwnedObjectPath = device.get_property("Ip4Config").await?;
    let address = ipv4_addresses(connection, ip4_config).await?.join(" ");

    let (ssid, signal_strength) = if wireless {
        active_wifi(connection, device_path).await?
    } else {
        (String::new(), 0)
    };

    Ok(Snapshot {
        connection_type: if wireless {
            ConnectionKind::Wifi
        } else {
            ConnectionKind::Ethernet
        },
        interface_name: interface.to_owned(),
        address,
        ssid,
        signal_strength,
    })
}

/// NetworkManager property changes, subscribed before the first snapshot is read.
pub struct PropertiesWatcher {
    changes: watch::Receiver<()>,
    task: JoinHandle<Result<()>>,
}

impl PropertiesWatcher {
    /// Drain NetworkManager changes even while snapshot property queries are pending.
    pub async fn open(connection: &Connection) -> Result<Self> {
        let rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .interface(PROPERTIES_INTERFACE)?
            .member("PropertiesChanged")?
            .sender(SERVICE)?
            .arg0ns(SERVICE)?
            .build();
        let mut messages = MessageStream::for_match_rule(rule, connection, Some(CHANGE_QUEUE))
            .await
            .context("subscribe to property changes")?;
        let (changed, changes) = watch::channel(());
        let task = tokio::task::spawn_local(async move {
            while let Some(message) = messages.next().await {
                let message = message.context("receive a property change")?;
                let Some(signal) = fdo::PropertiesChanged::from_message(message) else {
                    continue;
                };
                let arguments = signal.args()?;
                if arguments.interface_name().as_str().starts_with(SERVICE) {
                    changed.send_replace(());
                }
            }
            Ok(())
        });
        Ok(Self { changes, task })
    }

    /// Resolve once NetworkManager reported a property change.
    ///
    /// Returns `false` when the bus stops delivering messages.
    pub async fn next_change(&mut self) -> Result<bool> {
        if self.changes.changed().await.is_ok() {
            return Ok(true);
        }
        (&mut self.task).await??;
        Ok(false)
    }
}

impl Drop for PropertiesWatcher {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Build a proxy that reads every property from NetworkManager instead of a cache.
async fn proxy(
    connection: &Connection,
    path: ObjectPath<'static>,
    interface: &'static str,
) -> Result<Proxy<'static>> {
    Builder::new(connection)
        .destination(SERVICE)?
        .path(path)?
        .interface(interface)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(Into::into)
}

/// Object path of the NetworkManager device with this IP interface name.
async fn device_path(connection: &Connection, interface: &str) -> Result<Option<OwnedObjectPath>> {
    let manager = proxy(connection, MANAGER_PATH, MANAGER_INTERFACE).await?;
    let lookup: zbus::Result<OwnedObjectPath> =
        manager.call("GetDeviceByIpIface", &(interface,)).await;

    match lookup {
        Ok(path) => Ok(Some(path)),
        Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == UNKNOWN_DEVICE_ERROR => {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

/// All IPv4 addresses of the configuration, formatted as `address/prefix`.
async fn ipv4_addresses(connection: &Connection, path: OwnedObjectPath) -> Result<Vec<String>> {
    if path.as_str() == NO_OBJECT {
        return Ok(Vec::new());
    }

    let ip4_config = proxy(connection, path.into(), IP4_CONFIG_INTERFACE).await?;
    let addresses: Vec<FxHashMap<String, OwnedValue>> =
        ip4_config.get_property("AddressData").await?;

    addresses.iter().map(format_address).collect()
}

/// Format one `AddressData` entry (`a{sv}`) as `address/prefix`.
fn format_address(entry: &FxHashMap<String, OwnedValue>) -> Result<String> {
    let address = entry
        .get("address")
        .context("IPv4 address entry without an address")?;
    let prefix = entry
        .get("prefix")
        .context("IPv4 address entry without a prefix")?;

    let address: &str = address.try_into().context("IPv4 address is not a string")?;
    let prefix: u32 = prefix.try_into().context("IPv4 prefix is not a number")?;

    Ok(format!("{address}/{prefix}"))
}

/// SSID and signal strength of the access point the device is associated with.
async fn active_wifi(
    connection: &Connection,
    device_path: OwnedObjectPath,
) -> Result<(String, u8)> {
    let wireless = proxy(connection, device_path.into(), WIRELESS_INTERFACE).await?;
    let access_point: OwnedObjectPath = wireless.get_property("ActiveAccessPoint").await?;
    if access_point.as_str() == NO_OBJECT {
        return Ok((String::new(), 0));
    }

    let access_point = proxy(connection, access_point.into(), ACCESS_POINT_INTERFACE).await?;
    let ssid: Vec<u8> = access_point.get_property("Ssid").await?;
    let strength: u8 = access_point.get_property("Strength").await?;

    Ok((String::from_utf8_lossy(&ssid).into_owned(), strength))
}
