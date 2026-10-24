mod capture;
mod network_routes;
mod network_status;
mod resource_monitor;
mod system_info;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about, version)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Supervise the capture server across Quickshell reloads.
    Capture,
    #[command(hide = true)]
    CaptureServer,
    /// Select a Portal sharing source through the Quickshell interface.
    CapturePicker {
        #[arg(long)]
        allow_token: bool,
    },
    /// Stream the default IPv4 connection's status on routing and D-Bus events.
    NetworkStatus,
    /// Stream CPU, memory, or network metrics for an open resource panel.
    ResourceMonitor {
        #[arg(value_parser = ["cpu", "memory", "network"])]
        resource: String,
    },
    /// Print local software versions and the installed package count.
    SystemInfo,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let result = tokio::task::LocalSet::new().run_until(run()).await;
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) if output_disconnected(&error) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "[desktop-backend] {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn output_disconnected(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
        || error
            .downcast_ref::<serde_json::Error>()
            .is_some_and(|error| error.io_error_kind() == Some(io::ErrorKind::BrokenPipe))
}

async fn run() -> Result<bool> {
    match Arguments::parse().command {
        Command::Capture => capture::launch().await?,
        Command::CaptureServer => capture::serve().await?,
        Command::CapturePicker { allow_token } => return capture::pick(allow_token).await,
        Command::NetworkStatus => network_status::run().await?,
        Command::ResourceMonitor { resource } => resource_monitor::run(&resource).await?,
        Command::SystemInfo => system_info::run().await?,
    }
    Ok(true)
}
