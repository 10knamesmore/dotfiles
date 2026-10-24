//! Stream one resource panel's Linux metrics as compact JSON lines.

mod cpu;
mod memory;
mod network;
mod process;

use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use tokio::io::{AsyncWriteExt, Stdout};
use tokio::time::{Instant, MissedTickBehavior, interval_at};

const SAMPLE_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Serialize)]
struct Packet<'a> {
    resource: &'a str,
    time: f64,
    snapshot: &'a Snapshot,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Snapshot {
    Cpu(cpu::CpuSnapshot),
    Memory(memory::MemorySnapshot),
    Network(network::NetworkSnapshot),
}

enum Sampler {
    Cpu(cpu::CpuSampler),
    Memory(memory::MemorySampler),
    Network(network::NetworkSampler),
}

impl Sampler {
    async fn create(resource: &str) -> Result<Self> {
        Ok(match resource {
            "cpu" => Self::Cpu(tokio::task::spawn_blocking(cpu::CpuSampler::new).await??),
            "memory" => Self::Memory(memory::MemorySampler::new()),
            "network" => Self::Network(network::NetworkSampler::new().await?),
            other => bail!("unknown resource {other}"),
        })
    }

    async fn sample(self) -> Result<(Self, Snapshot)> {
        match self {
            Self::Cpu(mut sampler) => {
                tokio::task::spawn_blocking(move || {
                    let snapshot = Snapshot::Cpu(sampler.sample()?);
                    Ok((Self::Cpu(sampler), snapshot))
                })
                .await?
            }
            Self::Memory(mut sampler) => {
                tokio::task::spawn_blocking(move || {
                    let snapshot = Snapshot::Memory(sampler.sample()?);
                    Ok((Self::Memory(sampler), snapshot))
                })
                .await?
            }
            Self::Network(mut sampler) => {
                let snapshot = Snapshot::Network(sampler.sample().await?);
                Ok((Self::Network(sampler), snapshot))
            }
        }
    }
}

pub async fn run(resource: &str) -> Result<()> {
    let mut sampler = Sampler::create(resource).await?;
    let mut ticks = interval_at(Instant::now() + SAMPLE_INTERVAL, SAMPLE_INTERVAL);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut stdout = tokio::io::stdout();
    let mut line = Vec::with_capacity(1024);

    loop {
        ticks.tick().await;
        let (next_sampler, snapshot) = sampler.sample().await?;
        sampler = next_sampler;
        let packet = Packet {
            resource,
            time: epoch_seconds()?,
            snapshot: &snapshot,
        };
        line.clear();
        serde_json::to_writer(&mut line, &packet).context("serialize snapshot")?;
        line.push(b'\n');
        if let Err(error) = write_line(&mut stdout, &line).await {
            if error.kind() == io::ErrorKind::BrokenPipe {
                return Ok(());
            }
            return Err(error).context("write snapshot");
        }
    }
}

async fn write_line(stdout: &mut Stdout, line: &[u8]) -> io::Result<()> {
    stdout.write_all(line).await?;
    stdout.flush().await
}

fn epoch_seconds() -> Result<f64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?;
    Ok(elapsed.as_secs_f64())
}
