//! Status of the interface that owns the default IPv4 route.
//!
//! Kernel route notifications and NetworkManager property changes are the only triggers: the
//! stream never polls. A snapshot is written to stdout only when it differs from the previous one.

mod network_manager;
mod snapshot;

use std::io::{self, Write};

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use zbus::Connection;

use self::snapshot::Snapshot;
use crate::network_routes::NetworkRoutes;

/// Publish snapshots for the Quickshell network service until it closes the pipe.
pub async fn run() -> Result<()> {
    let connection = Connection::system()
        .await
        .context("connect to the system bus")?;
    // Subscribe before the initial read, so a change that lands while reading still triggers
    // another snapshot.
    let mut properties = network_manager::PropertiesWatcher::open(&connection).await?;
    let mut routes = NetworkRoutes::connect()?;

    let mut stdout = tokio::io::stdout();
    let mut published: Option<Snapshot> = None;

    loop {
        let routing = routes.snapshot().await?;
        let snapshot =
            network_manager::read(&connection, routing.default_interface.as_deref()).await?;
        if published.as_ref() != Some(&snapshot) {
            let mut line =
                serde_json::to_vec(&snapshot).context("serialize the status snapshot")?;
            line.push(b'\n');
            let output = async {
                stdout.write_all(&line).await?;
                stdout.flush().await
            }
            .await;
            if let Err(error) = output {
                if error.kind() == io::ErrorKind::BrokenPipe {
                    return Ok(());
                }
                return Err(error).context("write the status snapshot");
            }
            report(&snapshot);
            published = Some(snapshot);
        }

        tokio::select! {
            delivered = properties.next_change() => {
                if !delivered? {
                    return Ok(());
                }
            }
            changed = routes.changed() => changed?,
        }
    }
}

/// Log the connection that the published snapshot describes.
fn report(snapshot: &Snapshot) {
    let interface = if snapshot.interface_name.is_empty() {
        "none"
    } else {
        snapshot.interface_name.as_str()
    };
    let _ = writeln!(
        io::stderr().lock(),
        "[network-status] connection={} interface={interface}",
        snapshot.connection_type
    );
}
