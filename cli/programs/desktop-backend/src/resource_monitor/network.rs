use std::time::Instant;

use anyhow::Result;
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::network_routes::{NetworkRoutes, RoutingSnapshot};

#[derive(Serialize)]
pub struct NetworkSnapshot {
    #[serde(rename = "defaultInterface")]
    pub default_interface: String,
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ipv4: String,
    pub gateway: String,
    #[serde(rename = "downSpeed")]
    pub down_speed: f64,
    #[serde(rename = "upSpeed")]
    pub up_speed: f64,
    #[serde(rename = "downTotal")]
    pub down_total: u64,
    #[serde(rename = "upTotal")]
    pub up_total: u64,
}

#[derive(Clone, Copy)]
struct InterfaceCounters {
    received: u64,
    sent: u64,
}

pub struct NetworkSampler {
    previous: FxHashMap<String, InterfaceCounters>,
    previous_time: Instant,
    routes: NetworkRoutes,
    routing: RoutingSnapshot,
}

impl NetworkSampler {
    pub async fn new() -> Result<Self> {
        let mut routes = NetworkRoutes::connect()?;
        let routing = routes.snapshot().await?;
        let previous = tokio::task::spawn_blocking(read_counters).await??;
        Ok(Self {
            previous: previous.into_iter().collect(),
            previous_time: Instant::now(),
            routes,
            routing,
        })
    }

    pub async fn sample(&mut self) -> Result<NetworkSnapshot> {
        if self.routes.has_changed()? {
            self.routing = self.routes.snapshot().await?;
        }
        let counters = tokio::task::spawn_blocking(read_counters).await??;
        let now = Instant::now();
        let elapsed = now.duration_since(self.previous_time).as_secs_f64();
        let interfaces = counters
            .iter()
            .map(|(name, current)| {
                let previous = self.previous.get(name).copied().unwrap_or(*current);
                let routing = self
                    .routing
                    .interfaces
                    .iter()
                    .find(|interface| interface.name == *name);
                NetworkInterface {
                    name: name.clone(),
                    ipv4: routing
                        .map_or("", |interface| interface.ipv4.as_str())
                        .to_owned(),
                    gateway: routing
                        .map_or("", |interface| interface.gateway.as_str())
                        .to_owned(),
                    down_speed: speed(current.received.saturating_sub(previous.received), elapsed),
                    up_speed: speed(current.sent.saturating_sub(previous.sent), elapsed),
                    down_total: current.received,
                    up_total: current.sent,
                }
            })
            .collect();
        self.previous = counters.into_iter().collect();
        self.previous_time = now;
        Ok(NetworkSnapshot {
            default_interface: self.routing.default_interface.clone().unwrap_or_default(),
            interfaces,
        })
    }
}

fn read_counters() -> Result<Vec<(String, InterfaceCounters)>> {
    let mut counters: Vec<_> = procfs::net::dev_status()?
        .into_iter()
        .filter(|(name, _)| name != "lo")
        .map(|(name, status)| {
            (
                name,
                InterfaceCounters {
                    received: status.recv_bytes,
                    sent: status.sent_bytes,
                },
            )
        })
        .collect();
    counters.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(counters)
}

fn speed(bytes: u64, elapsed: f64) -> f64 {
    if elapsed > 0.0 {
        bytes as f64 / elapsed
    } else {
        0.0
    }
}
