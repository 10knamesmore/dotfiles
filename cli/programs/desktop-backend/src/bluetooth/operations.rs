use std::sync::{Arc, Mutex, atomic::Ordering};

use anyhow::{Context, Result, ensure};
use rustc_hash::{FxHashMap, FxHashSet};
use zbus::{
    Connection, Proxy,
    proxy::{Builder, CacheProperties},
    zvariant::{ObjectPath, OwnedObjectPath},
};

use super::{
    agent::{self, Agent},
    protocol::{Action, Operation, Output, Stage},
    snapshot::{ADAPTER, DEVICE},
};

#[derive(Clone, Copy)]
enum ConnectionPhase {
    Preparing,
    Pairing,
    Connecting,
}

struct ConnectionAttempt {
    phase: ConnectionPhase,
    canceled: bool,
}

#[derive(Default)]
struct Work {
    connections: FxHashMap<OwnedObjectPath, ConnectionAttempt>,
    discovery: FxHashSet<OwnedObjectPath>,
}

#[derive(Clone)]
pub struct Session {
    pub connection: Connection,
    pub agent: Agent,
    pub output: Output,
    work: Arc<Mutex<Work>>,
}

impl Session {
    pub fn new(connection: Connection, agent: Agent, output: Output) -> Self {
        Self {
            connection,
            agent,
            output,
            work: Arc::default(),
        }
    }

    pub async fn perform(&self, operation: Operation) -> Result<()> {
        let result = self.execute(&operation).await;
        match result {
            Ok(()) => self.output.operation(&operation, Stage::Done, None),
            Err(error) => {
                self.output
                    .operation(&operation, Stage::Error, Some(format!("{error:#}")))
            }
        }
    }

    async fn execute(&self, operation: &Operation) -> Result<()> {
        match operation.action {
            Action::Power => {
                let adapter = operation
                    .adapter
                    .as_ref()
                    .context("power requires adapter")?;
                let value = operation.value.context("power requires value")?;
                proxy(&self.connection, adapter, ADAPTER)
                    .await?
                    .set_property("Powered", value)
                    .await?;
            }
            Action::Scan => {
                let adapter = operation
                    .adapter
                    .as_ref()
                    .context("scan requires adapter")?;
                let value = operation.value.context("scan requires value")?;
                let adapter_proxy = proxy(&self.connection, adapter, ADAPTER).await?;
                if value {
                    adapter_proxy
                        .call::<_, _, ()>("StartDiscovery", &())
                        .await?;
                    self.work.lock().unwrap().discovery.insert(adapter.clone());
                } else {
                    adapter_proxy.call::<_, _, ()>("StopDiscovery", &()).await?;
                    self.work.lock().unwrap().discovery.remove(adapter);
                }
            }
            _ => {
                let device = operation
                    .device
                    .as_ref()
                    .context("action requires device")?;
                match operation.action {
                    Action::Connect => {
                        self.work.lock().unwrap().connections.insert(
                            device.clone(),
                            ConnectionAttempt {
                                phase: ConnectionPhase::Preparing,
                                canceled: false,
                            },
                        );
                        let result = self.connect(operation, device).await;
                        self.agent.close_device(device);
                        self.work.lock().unwrap().connections.remove(device);
                        result?;
                    }
                    Action::Disconnect => {
                        proxy(&self.connection, device, DEVICE)
                            .await?
                            .call::<_, _, ()>("Disconnect", &())
                            .await?;
                    }
                    Action::Cancel => self.cancel(device).await?,
                    Action::Forget => {
                        let adapter: OwnedObjectPath = proxy(&self.connection, device, DEVICE)
                            .await?
                            .get_property("Adapter")
                            .await?;
                        proxy(&self.connection, &adapter, ADAPTER)
                            .await?
                            .call::<_, _, ()>("RemoveDevice", &(device,))
                            .await?;
                    }
                    Action::Trust => {
                        let value = operation.value.context("trust requires value")?;
                        proxy(&self.connection, device, DEVICE)
                            .await?
                            .set_property("Trusted", value)
                            .await?;
                    }
                    Action::Power | Action::Scan => unreachable!(),
                }
            }
        }
        Ok(())
    }

    async fn connect(&self, operation: &Operation, device: &OwnedObjectPath) -> Result<()> {
        let device_proxy = proxy(&self.connection, device, DEVICE).await?;
        let paired: bool = device_proxy.get_property("Paired").await?;
        let bonded: bool = device_proxy.get_property("Bonded").await?;
        if !paired && !bonded {
            self.stage(device, ConnectionPhase::Pairing)?;
            self.output.operation(operation, Stage::Pairing, None)?;
            let result = device_proxy.call::<_, _, ()>("Pair", &()).await;
            // BlueZ may finish Pair without sending Cancel for display-only prompts.
            self.agent.close_device(device);
            result?;
        }
        self.stage(device, ConnectionPhase::Connecting)?;
        self.output.operation(operation, Stage::Connecting, None)?;
        device_proxy.call::<_, _, ()>("Connect", &()).await?;
        Ok(())
    }

    fn stage(&self, device: &OwnedObjectPath, phase: ConnectionPhase) -> Result<()> {
        let mut work = self.work.lock().unwrap();
        let attempt = work.connections.get_mut(device).unwrap();
        ensure!(!attempt.canceled, "Bluetooth operation canceled by user");
        attempt.phase = phase;
        Ok(())
    }

    async fn cancel(&self, device: &OwnedObjectPath) -> Result<()> {
        let phase = {
            let mut work = self.work.lock().unwrap();
            work.connections.get_mut(device).map(|attempt| {
                attempt.canceled = true;
                attempt.phase
            })
        };
        let method = match phase {
            Some(ConnectionPhase::Connecting) => "Disconnect",
            _ => "CancelPairing",
        };
        let result = async {
            proxy(&self.connection, device, DEVICE)
                .await?
                .call::<_, _, ()>(method, &())
                .await?;
            Ok(())
        }
        .await;
        self.agent.close_device(device);
        result
    }

    pub async fn register_agent(&self) -> Result<()> {
        self.connection
            .object_server()
            .at(agent::PATH, self.agent.clone())
            .await?;
        let path = ObjectPath::try_from(agent::PATH)?;
        proxy(
            &self.connection,
            &OwnedObjectPath::try_from("/org/bluez")?,
            "org.bluez.AgentManager1",
        )
        .await?
        .call::<_, _, ()>("RegisterAgent", &(path, "KeyboardDisplay"))
        .await?;
        self.agent.registered.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub async fn cleanup(&self) {
        self.agent.close_all();
        let (connections, discovery) = {
            let mut work = self.work.lock().unwrap();
            (
                std::mem::take(&mut work.connections),
                std::mem::take(&mut work.discovery),
            )
        };
        for (device, attempt) in connections {
            let method = match attempt.phase {
                ConnectionPhase::Preparing => continue,
                ConnectionPhase::Pairing => "CancelPairing",
                ConnectionPhase::Connecting => "Disconnect",
            };
            let result = async {
                proxy(&self.connection, &device, DEVICE)
                    .await?
                    .call::<_, _, ()>(method, &())
                    .await?;
                Ok::<_, anyhow::Error>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("[bluetooth] {method} {device}: {error:#}");
            }
        }
        for adapter in discovery {
            let result = async {
                proxy(&self.connection, &adapter, ADAPTER)
                    .await?
                    .call::<_, _, ()>("StopDiscovery", &())
                    .await?;
                Ok::<_, anyhow::Error>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("[bluetooth] StopDiscovery {adapter}: {error:#}");
            }
        }
        if self.agent.registered.swap(false, Ordering::SeqCst) {
            let result = async {
                proxy(
                    &self.connection,
                    &OwnedObjectPath::try_from("/org/bluez")?,
                    "org.bluez.AgentManager1",
                )
                .await?
                .call::<_, _, ()>("UnregisterAgent", &(ObjectPath::try_from(agent::PATH)?,))
                .await?;
                Ok::<_, anyhow::Error>(())
            }
            .await;
            if let Err(error) = result {
                eprintln!("[bluetooth] UnregisterAgent: {error:#}");
            }
        }
    }
}

async fn proxy(
    connection: &Connection,
    path: &OwnedObjectPath,
    interface: &'static str,
) -> Result<Proxy<'static>> {
    Ok(Builder::new(connection)
        .destination("org.bluez")?
        .path(path.clone())?
        .interface(interface)?
        .cache_properties(CacheProperties::No)
        .build()
        .await?)
}
