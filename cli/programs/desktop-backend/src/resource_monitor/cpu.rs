use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use procfs::{CurrentSI, KernelStats};
use rustc_hash::FxHashMap;
use serde::Serialize;

use super::process::{ProcessSample, scan_processes};

const PROCESS_INTERVAL: Duration = Duration::from_millis(500);
const HARDWARE_INTERVAL: Duration = Duration::from_secs(1);
const CPU_SENSOR_NAMES: [&str; 4] = ["k10temp", "coretemp", "zenpower", "cpu_thermal"];
const PACKAGE_LABELS: [&str; 4] = ["Tctl", "Tdie", "Package id 0", "Package id 1"];

#[derive(Serialize)]
pub struct CpuSnapshot {
    pub usage: f64,
    #[serde(rename = "frequencyGHz")]
    pub frequency_ghz: Option<f64>,
    #[serde(rename = "temperatureC")]
    pub temperature_c: Option<f64>,
    pub processes: Vec<CpuProcess>,
}

#[derive(Clone, Serialize)]
pub struct CpuProcess {
    pub pid: u32,
    pub name: String,
    pub usage: f64,
}

#[derive(Clone, Copy)]
struct CpuTimes {
    total: u64,
    idle: u64,
}

pub struct CpuSampler {
    previous_cpu: CpuTimes,
    process_cpu_total: u64,
    processes: FxHashMap<u32, ProcessSample>,
    process_time: Option<Instant>,
    ranking: Vec<CpuProcess>,
    frequency_paths: Vec<PathBuf>,
    temperature_paths: Vec<PathBuf>,
    frequency_ghz: Option<f64>,
    temperature_c: Option<f64>,
    hardware_time: Option<Instant>,
}

impl CpuSampler {
    pub fn new() -> Result<Self> {
        let cpu = read_cpu_times()?;
        Ok(Self {
            previous_cpu: cpu,
            process_cpu_total: cpu.total,
            processes: scan_processes()?
                .into_iter()
                .map(|process| (process.pid, process))
                .collect(),
            process_time: None,
            ranking: Vec::new(),
            frequency_paths: discover_frequency_paths()?,
            temperature_paths: discover_temperature_paths()?,
            frequency_ghz: None,
            temperature_c: None,
            hardware_time: None,
        })
    }

    pub fn sample(&mut self) -> Result<CpuSnapshot> {
        let now = Instant::now();
        let current = read_cpu_times()?;
        if self
            .process_time
            .is_none_or(|time| now.duration_since(time) >= PROCESS_INTERVAL)
        {
            self.refresh_processes(current.total)?;
            self.process_time = Some(now);
        }
        if self
            .hardware_time
            .is_none_or(|time| now.duration_since(time) >= HARDWARE_INTERVAL)
        {
            self.refresh_hardware()?;
            self.hardware_time = Some(now);
        }
        let snapshot = CpuSnapshot {
            usage: cpu_usage(current, self.previous_cpu),
            frequency_ghz: self.frequency_ghz,
            temperature_c: self.temperature_c,
            processes: self.ranking.clone(),
        };
        self.previous_cpu = current;
        Ok(snapshot)
    }

    /// The jiffy delta is divided by the whole-machine jiffies elapsed since the previous
    /// scan, so the leaderboard stays on the machine-wide 100% scale at its slower cadence.
    fn refresh_processes(&mut self, cpu_total: u64) -> Result<()> {
        let sampled = scan_processes()?;
        let elapsed = cpu_total.saturating_sub(self.process_cpu_total);
        let mut ranked = Vec::new();
        for process in &sampled {
            let Some(previous) = self.processes.get(&process.pid) else {
                continue;
            };
            // A reused PID belongs to a different process; it has no previous sample.
            if previous.started != process.started {
                continue;
            }
            let usage = if elapsed == 0 {
                0.0
            } else {
                100.0 * process.cpu_ticks.saturating_sub(previous.cpu_ticks) as f64 / elapsed as f64
            };
            ranked.push(CpuProcess {
                pid: process.pid,
                name: process.name.clone(),
                usage,
            });
        }
        self.processes = sampled
            .into_iter()
            .map(|process| (process.pid, process))
            .collect();
        self.process_cpu_total = cpu_total;
        ranked.sort_by(|left, right| right.usage.total_cmp(&left.usage));
        ranked.truncate(8);
        self.ranking = ranked;
        Ok(())
    }

    fn refresh_hardware(&mut self) -> Result<()> {
        let mut frequencies = Vec::new();
        for path in &self.frequency_paths {
            if let Some(kilohertz) = optional_number(path)? {
                frequencies.push(kilohertz / 1_000_000.0);
            }
        }
        let mut temperatures = Vec::new();
        for path in &self.temperature_paths {
            if let Some(millidegrees) = optional_number(path)? {
                temperatures.push(millidegrees / 1000.0);
            }
        }
        self.frequency_ghz = average(&frequencies);
        self.temperature_c = temperatures.into_iter().reduce(f64::max);
        Ok(())
    }
}

fn read_cpu_times() -> Result<CpuTimes> {
    let cpu = KernelStats::current()?.total;
    let iowait = cpu.iowait.context("CPU times have no iowait")?;
    // Guest and guest_nice are already included in user and nice.
    Ok(CpuTimes {
        total: cpu.user
            + cpu.nice
            + cpu.system
            + cpu.idle
            + iowait
            + cpu.irq.context("CPU times have no irq")?
            + cpu.softirq.context("CPU times have no softirq")?
            + cpu.steal.context("CPU times have no steal")?,
        idle: cpu.idle + iowait,
    })
}

fn cpu_usage(current: CpuTimes, previous: CpuTimes) -> f64 {
    let elapsed = current.total.saturating_sub(previous.total);
    if elapsed == 0 {
        return 0.0;
    }
    let busy = elapsed as f64 - current.idle as f64 + previous.idle as f64;
    100.0 * busy / elapsed as f64
}

fn discover_frequency_paths() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in optional_entries(Path::new("/sys/devices/system/cpu/cpufreq"))? {
        if !entry.file_name().to_string_lossy().starts_with("policy") {
            continue;
        }
        let path = entry.path().join("scaling_cur_freq");
        if path.is_file() {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn discover_temperature_paths() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for monitor in optional_entries(Path::new("/sys/class/hwmon"))? {
        if !monitor.file_name().to_string_lossy().starts_with("hwmon") {
            continue;
        }
        let name_path = monitor.path().join("name");
        let name = fs::read_to_string(&name_path)
            .with_context(|| format!("read {}", name_path.display()))?;
        if !CPU_SENSOR_NAMES.contains(&name.trim()) {
            continue;
        }
        for sensor in optional_entries(&monitor.path())? {
            let file_name = sensor.file_name();
            let Some(file_name) = file_name.to_str() else {
                continue;
            };
            if !(file_name.starts_with("temp") && file_name.ends_with("_input")) {
                continue;
            }
            let label_path = sensor
                .path()
                .with_file_name(file_name.replace("_input", "_label"));
            if label_path.exists() {
                let label = fs::read_to_string(&label_path)
                    .with_context(|| format!("read {}", label_path.display()))?;
                if !PACKAGE_LABELS.contains(&label.trim()) {
                    continue;
                }
            }
            paths.push(sensor.path());
        }
    }
    Ok(paths)
}

/// Hardware directories are optional: a kernel without cpufreq or hwmon simply has none.
fn optional_entries(directory: &Path) -> Result<Vec<fs::DirEntry>> {
    match fs::read_dir(directory) {
        Ok(entries) => entries
            .collect::<io::Result<Vec<_>>>()
            .with_context(|| format!("read {}", directory.display())),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            Ok(Vec::new())
        }
        Err(error) => Err(error).with_context(|| format!("read {}", directory.display())),
    }
}

fn optional_number(path: &Path) -> Result<Option<f64>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    Ok(Some(
        text.trim()
            .parse()
            .with_context(|| format!("parse {}", path.display()))?,
    ))
}

fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}
