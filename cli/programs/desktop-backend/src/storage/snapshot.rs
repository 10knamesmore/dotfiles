use std::collections::BTreeMap;

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;
use serde::Serialize;
use zbus::{
    Connection,
    names::OwnedInterfaceName,
    zvariant::{OwnedObjectPath, OwnedValue},
};

pub const SERVICE: &str = "org.freedesktop.UDisks2";
pub const ROOT: &str = "/org/freedesktop/UDisks2";
pub const DRIVE: &str = "org.freedesktop.UDisks2.Drive";
pub const BLOCK: &str = "org.freedesktop.UDisks2.Block";
pub const FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";
const PARTITION: &str = "org.freedesktop.UDisks2.Partition";
const TABLE: &str = "org.freedesktop.UDisks2.PartitionTable";

type Properties = FxHashMap<String, OwnedValue>;
type Interfaces = FxHashMap<OwnedInterfaceName, Properties>;
type ManagedObjects = FxHashMap<OwnedObjectPath, Interfaces>;

#[zbus::proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.freedesktop.UDisks2",
    default_path = "/org/freedesktop/UDisks2"
)]
trait ObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    pub path: String,
    pub name: String,
    pub connection_bus: String,
    pub size: u64,
    pub can_power_off: bool,
    pub ejectable: bool,
    pub volumes: Vec<Volume>,
    #[serde(skip)]
    pub media_detected: u64,
    #[serde(skip)]
    pub sibling_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub path: String,
    pub key: String,
    pub device: String,
    pub label: String,
    pub uuid: String,
    pub filesystem: String,
    pub size: u64,
    pub read_only: bool,
    pub mountable: bool,
    pub mount_points: Vec<String>,
    #[serde(skip)]
    pub fstab_read_only: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub path: String,
    pub drive: String,
    pub mount_points: Vec<String>,
    pub active_swap: bool,
    pub cleartext_device: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub drives: Vec<Drive>,
    // Includes hidden blocks so safe removal never overlooks their mounted filesystems.
    pub blocks: Vec<Block>,
    pub block_paths: Vec<String>,
}

impl Snapshot {
    pub async fn read(connection: &Connection) -> Result<Self> {
        // One ObjectManager read avoids GetAll races with disappearing block objects.
        let objects = ObjectManagerProxy::new(connection)
            .await?
            .get_managed_objects()
            .await?;
        let mut non_system_drives = rustc_hash::FxHashSet::default();
        for interfaces in objects.values() {
            if let Some(block) = interfaces.get(BLOCK)
                && !property::<bool>(block, "HintSystem")?
                && !property::<bool>(block, "HintIgnore")?
            {
                non_system_drives.insert(property::<OwnedObjectPath>(block, "Drive")?);
            }
        }
        let mut drives = BTreeMap::new();
        for (path, interfaces) in &objects {
            let Some(properties) = interfaces.get(DRIVE) else {
                continue;
            };
            let connection_bus: String = property(properties, "ConnectionBus")?;
            let media: String = property(properties, "Media")?;
            let removable = property::<bool>(properties, "Removable")?
                || property::<bool>(properties, "MediaRemovable")?;
            let sd_card = media.starts_with("flash_sd");
            // SD readers can report RM=0. An sdio bus alone also matches internal eMMC.
            let non_system_sdio = connection_bus == "sdio" && non_system_drives.contains(path);
            if !(connection_bus == "usb" || removable || sd_card || non_system_sdio)
                || !property::<bool>(properties, "MediaAvailable")?
            {
                continue;
            }
            let vendor: String = property(properties, "Vendor")?;
            let model: String = property(properties, "Model")?;
            let name = [vendor.trim(), model.trim()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            drives.insert(
                path.to_string(),
                Drive {
                    path: path.to_string(),
                    name: if name.is_empty() {
                        "Removable drive".into()
                    } else {
                        name
                    },
                    connection_bus,
                    size: property(properties, "Size")?,
                    can_power_off: property(properties, "CanPowerOff")?,
                    ejectable: property(properties, "Ejectable")?,
                    volumes: Vec::new(),
                    media_detected: property(properties, "TimeMediaDetected")?,
                    sibling_id: property(properties, "SiblingId")?,
                },
            );
        }
        let mut snapshot = Self::default();
        for (path, interfaces) in &objects {
            let Some(properties) = interfaces.get(BLOCK) else {
                continue;
            };
            snapshot.block_paths.push(path.to_string());
            let drive_path = block_drive(path, &objects)?;
            let filesystem = interfaces.get(FILESYSTEM);
            let mount_points = filesystem
                .map(|properties| property::<Vec<Vec<u8>>>(properties, "MountPoints"))
                .transpose()?
                .unwrap_or_default()
                .into_iter()
                .map(decode_path)
                .collect::<Vec<_>>();
            let crypto: OwnedObjectPath = property(properties, "CryptoBackingDevice")?;
            snapshot.blocks.push(Block {
                path: path.to_string(),
                drive: drive_path.clone(),
                mount_points: mount_points.clone(),
                active_swap: interfaces
                    .get("org.freedesktop.UDisks2.Swapspace")
                    .map(|properties| property(properties, "Active"))
                    .transpose()?
                    .unwrap_or(false),
                cleartext_device: crypto.as_str() != "/",
            });
            let Some(drive) = drives.get_mut(&drive_path) else {
                continue;
            };
            if property::<bool>(properties, "HintIgnore")?
                || (interfaces.contains_key(TABLE)
                    && !interfaces.contains_key(PARTITION)
                    && filesystem.is_none())
            {
                continue;
            }
            let uuid: String = property(properties, "IdUUID")?;
            drive.volumes.push(Volume {
                path: path.to_string(),
                key: uuid.clone(),
                device: decode_path(property(properties, "PreferredDevice")?),
                label: property(properties, "IdLabel")?,
                uuid,
                filesystem: property(properties, "IdType")?,
                size: property(properties, "Size")?,
                read_only: property(properties, "ReadOnly")?,
                mountable: filesystem.is_some(),
                mount_points,
                fstab_read_only: fstab_read_only(properties)?,
            });
        }
        for drive in drives.values_mut() {
            drive
                .volumes
                .sort_by(|left, right| left.path.cmp(&right.path));
        }
        snapshot.drives = drives.into_values().collect();
        Ok(snapshot)
    }

    pub fn volume(&self, path: &str) -> Option<(&Drive, &Volume)> {
        self.drives.iter().find_map(|drive| {
            drive
                .volumes
                .iter()
                .find(|volume| volume.path == path)
                .map(|volume| (drive, volume))
        })
    }
}

fn block_drive(path: &OwnedObjectPath, objects: &ManagedObjects) -> Result<String> {
    let mut current = path;
    // Cleartext mappings can have Drive="/"; follow their backing block to the physical drive.
    for _ in 0..objects.len() {
        let Some(block) = objects
            .get(current)
            .and_then(|interfaces| interfaces.get(BLOCK))
        else {
            return Ok(String::new());
        };
        let drive: OwnedObjectPath = property(block, "Drive")?;
        if drive.as_str() != "/" {
            return Ok(drive.to_string());
        }
        let backing: OwnedObjectPath = property(block, "CryptoBackingDevice")?;
        let Some((path, _)) = objects.get_key_value(&backing) else {
            return Ok(String::new());
        };
        current = path;
    }
    Ok(String::new())
}

fn decode_path(mut bytes: Vec<u8>) -> String {
    if bytes.last() == Some(&0) {
        bytes.pop();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn fstab_read_only(properties: &Properties) -> Result<Option<bool>> {
    let configuration: Vec<(String, Properties)> = property(properties, "Configuration")?;
    for (kind, details) in configuration {
        if kind == "fstab" {
            let options = decode_path(property(&details, "opts")?);
            return Ok(Some(options.split(',').fold(
                false,
                |read_only, option| match option {
                    "ro" => true,
                    "rw" => false,
                    _ => read_only,
                },
            )));
        }
    }
    Ok(None)
}

fn property<T>(properties: &Properties, name: &str) -> Result<T>
where
    T: TryFrom<OwnedValue>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let value = properties
        .get(name)
        .with_context(|| format!("UDisks2 property {name} is missing"))?;
    T::try_from(value.try_clone()?).with_context(|| format!("read UDisks2 property {name}"))
}
