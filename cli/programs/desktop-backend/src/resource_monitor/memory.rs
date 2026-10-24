use std::cmp::Reverse;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use procfs::{Current, Meminfo, MemoryPressure, ProcError};
use serde::Serialize;

use super::process::scan_processes;

const PROCESS_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Serialize)]
pub struct MemorySnapshot {
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "usedBytes")]
    pub used_bytes: u64,
    #[serde(rename = "availableBytes")]
    pub available_bytes: u64,
    pub breakdown: MemoryBreakdown,
    #[serde(rename = "swapTotalBytes")]
    pub swap_total_bytes: u64,
    #[serde(rename = "swapUsedBytes")]
    pub swap_used_bytes: u64,
    #[serde(rename = "pressureSome10")]
    pub pressure_some10: Option<f64>,
    pub processes: Vec<MemoryProcess>,
}

#[derive(Serialize)]
pub struct MemoryBreakdown {
    #[serde(rename = "freeBytes")]
    pub free_bytes: u64,
    #[serde(rename = "buffersBytes")]
    pub buffers_bytes: u64,
    #[serde(rename = "cachedBytes")]
    pub cached_bytes: u64,
    #[serde(rename = "sharedBytes")]
    pub shared_bytes: u64,
}

#[derive(Clone, Serialize)]
pub struct MemoryProcess {
    pub pid: u32,
    pub name: String,
    #[serde(rename = "rssBytes")]
    pub rss_bytes: u64,
}

pub struct MemorySampler {
    process_time: Option<Instant>,
    processes: Vec<MemoryProcess>,
}

impl MemorySampler {
    pub fn new() -> Self {
        Self {
            process_time: None,
            processes: Vec::new(),
        }
    }

    pub fn sample(&mut self) -> Result<MemorySnapshot> {
        let now = Instant::now();
        let info = Meminfo::current()?;
        let pressure = read_memory_pressure()?;
        if self
            .process_time
            .is_none_or(|time| now.duration_since(time) >= PROCESS_INTERVAL)
        {
            let mut largest = scan_processes()?;
            largest.sort_by_key(|process| Reverse(process.rss_bytes));
            largest.truncate(8);
            self.processes = largest
                .into_iter()
                .map(|process| MemoryProcess {
                    pid: process.pid,
                    name: process.name,
                    rss_bytes: process.rss_bytes,
                })
                .collect();
            self.process_time = Some(now);
        }
        let available = info
            .mem_available
            .context("memory statistics have no MemAvailable")?;
        Ok(MemorySnapshot {
            total_bytes: info.mem_total,
            used_bytes: info.mem_total - available,
            available_bytes: available,
            breakdown: MemoryBreakdown {
                free_bytes: info.mem_free,
                buffers_bytes: info.buffers,
                cached_bytes: info.cached,
                shared_bytes: info.shmem.context("memory statistics have no Shmem")?,
            },
            swap_total_bytes: info.swap_total,
            swap_used_bytes: info.swap_total - info.swap_free,
            pressure_some10: pressure,
            processes: self.processes.clone(),
        })
    }
}

fn read_memory_pressure() -> Result<Option<f64>> {
    match MemoryPressure::current() {
        Ok(pressure) => Ok(Some(f64::from(pressure.some.avg10))),
        Err(ProcError::NotFound(_)) => Ok(None),
        Err(error) => Err(error).context("read memory pressure"),
    }
}
