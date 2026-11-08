use anyhow::{Context, Result, ensure};
use rustc_hash::FxHashMap;
use zbus::{
    Connection, Proxy,
    proxy::{Builder, CacheProperties},
    zvariant::Value,
};

use super::{
    protocol::{Action, Command},
    settings::Settings,
    snapshot::{DRIVE, Drive, FILESYSTEM, SERVICE, Snapshot},
};

pub struct AutomaticMount {
    pub drive: String,
    pub media_detected: u64,
    pub uuid: String,
}

pub async fn execute(
    connection: &Connection,
    command: &Command,
    settings: &Settings,
    automatic: Option<&AutomaticMount>,
) -> Result<()> {
    let snapshot = Snapshot::read(connection).await?;
    match &command.action {
        Action::Mount { target, read_only } => {
            let (drive, volume) = snapshot
                .volume(target)
                .context("volume is no longer available")?;
            if let Some(expected) = automatic {
                ensure!(
                    drive.path == expected.drive
                        && drive.media_detected == expected.media_detected
                        && volume.uuid == expected.uuid,
                    "inserted volume is no longer available"
                );
                ensure!(
                    settings.should_mount(&volume.key),
                    "automatic mounting is now disabled"
                );
            }
            ensure!(volume.mountable, "volume has no mountable filesystem");
            ensure!(volume.mount_points.is_empty(), "volume is already mounted");
            let read_only = volume.read_only
                || read_only.unwrap_or_else(|| settings.preference(&volume.key).read_only);
            // UDisks ignores caller mount options for entries configured in /etc/fstab.
            ensure!(
                !read_only || volume.fstab_read_only != Some(false),
                "read-only mounting requires ro in this volume's /etc/fstab options"
            );
            let mut options = FxHashMap::default();
            if read_only {
                options.insert("options", Value::from("ro"));
            }
            proxy(connection, target, FILESYSTEM)
                .await?
                .call::<_, _, String>("Mount", &(options,))
                .await?;
        }
        Action::Unmount { target } => {
            let (_, volume) = snapshot
                .volume(target)
                .context("volume is no longer available")?;
            ensure!(volume.mountable, "volume has no mountable filesystem");
            ensure!(!volume.mount_points.is_empty(), "volume is not mounted");
            unmount(connection, target).await?;
        }
        Action::Remove { target } => {
            let drives = removal_drives(&snapshot, target)?;
            let drive = drives[0];
            let blocks = snapshot
                .blocks
                .iter()
                .filter(|block| drives.iter().any(|drive| drive.path == block.drive))
                .collect::<Vec<_>>();
            for block in &blocks {
                ensure!(
                    !block.active_swap,
                    "{} is active swap; deactivate it before removing the drive",
                    block.path
                );
                ensure!(
                    !block.cleartext_device,
                    "{} is an unlocked encrypted volume; unmount and lock it before removing the drive",
                    block.path
                );
            }
            // Nested mount points must be unmounted before their parents.
            let mut mounted = blocks
                .into_iter()
                .filter(|block| !block.mount_points.is_empty())
                .collect::<Vec<_>>();
            mounted.sort_by_key(|block| {
                std::cmp::Reverse(
                    block
                        .mount_points
                        .iter()
                        .map(|path| path.matches('/').count())
                        .max()
                        .unwrap_or(0),
                )
            });
            for block in mounted {
                unmount(connection, &block.path)
                    .await
                    .with_context(|| format!("unmount {} before safe removal", block.path))?;
            }
            if drive.can_power_off {
                drive_method(connection, &drive.path, "PowerOff").await?;
            } else if drive.ejectable {
                drive_method(connection, &drive.path, "Eject").await?;
            }
        }
        Action::Settings { .. } | Action::Preference { .. } => unreachable!(),
    }
    Ok(())
}

pub fn removal_drives<'a>(snapshot: &'a Snapshot, target: &str) -> Result<Vec<&'a Drive>> {
    let drive = snapshot
        .drives
        .iter()
        .find(|drive| drive.path == target)
        .context("drive is no longer available")?;
    let mut drives = vec![drive];
    // PowerOff can also disconnect the other slots of a multi-card USB reader.
    if drive.can_power_off && !drive.sibling_id.is_empty() {
        drives.extend(
            snapshot
                .drives
                .iter()
                .filter(|other| other.path != drive.path && other.sibling_id == drive.sibling_id),
        );
    }
    Ok(drives)
}

async fn unmount(connection: &Connection, path: &str) -> Result<()> {
    proxy(connection, path, FILESYSTEM)
        .await?
        .call::<_, _, ()>("Unmount", &(FxHashMap::<&str, Value<'_>>::default(),))
        .await?;
    Ok(())
}

async fn drive_method(connection: &Connection, path: &str, method: &str) -> Result<()> {
    proxy(connection, path, DRIVE)
        .await?
        .call::<_, _, ()>(method, &(FxHashMap::<&str, Value<'_>>::default(),))
        .await?;
    Ok(())
}

async fn proxy(
    connection: &Connection,
    path: &str,
    interface: &'static str,
) -> Result<Proxy<'static>> {
    Ok(Builder::new(connection)
        .destination(SERVICE)?
        .path(path.to_owned())?
        .interface(interface)?
        .cache_properties(CacheProperties::No)
        .build()
        .await?)
}
