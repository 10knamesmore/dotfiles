//! Process samples shared by the CPU and memory leaderboards.

use anyhow::{Context, Result};
use procfs::{ProcError, process::all_processes};

/// A process as seen in one scan. CPU ticks exclude children; start time identifies PID reuse.
pub struct ProcessSample {
    pub pid: u32,
    pub name: String,
    pub cpu_ticks: u64,
    pub started: u64,
    pub rss_bytes: u64,
}

pub fn scan_processes() -> Result<Vec<ProcessSample>> {
    let page_bytes = procfs::page_size();
    let mut processes = Vec::new();
    for process in all_processes().context("list processes")? {
        let stat = match process.and_then(|process| process.stat()) {
            Ok(stat) => stat,
            // Processes may exit during the scan; hidepid can deny access to their stats.
            Err(ProcError::NotFound(_) | ProcError::PermissionDenied(_)) => continue,
            Err(error) => return Err(error).context("read process statistics"),
        };
        processes.push(ProcessSample {
            pid: stat.pid as u32,
            name: stat.comm,
            cpu_ticks: stat.utime + stat.stime,
            started: stat.starttime,
            rss_bytes: stat.rss * page_bytes,
        });
    }
    Ok(processes)
}
