mod color;
mod geometry;
mod macos;
mod manager;
mod paint;
mod stack;

use std::time::{Duration, Instant};

use anyhow::{Result, ensure};
use clap::Parser;
use color::Color;
use geometry::Layout;
use macos::{Connection, events::Events};
use manager::{Settings, WindowManager};
use stack::StackReader;

#[derive(Parser)]
#[command(about = "Low-memory macOS window borders using four narrow surfaces")]
struct Options {
    /// Manage only this window; omitted means all document and modal windows.
    #[arg(long)]
    window: Option<u32>,
    /// Full and empty are allocation-comparison modes.
    #[arg(long, value_enum, default_value = "strips")]
    layout: Layout,
    /// Border width in points.
    #[arg(long, default_value_t = 3.0)]
    width: f64,
    /// Target corner radius in points.
    #[arg(long, default_value_t = 9.0)]
    radius: f64,
    /// Override pixels per point; omitted follows the target display.
    #[arg(long)]
    scale: Option<f64>,
    /// Focused-window ARGB color.
    #[arg(long, default_value = "0xffcba6f7")]
    active_color: Color,
    /// Unfocused-window ARGB color.
    #[arg(long, default_value = "0xff45475a")]
    inactive_color: Color,
    /// Exit after this duration; omitted runs until Ctrl-C or SIGTERM.
    #[arg(long)]
    seconds: Option<u64>,
    /// Print window lifecycle, focus, surface allocation, and display changes.
    #[arg(long)]
    verbose: bool,
    /// Segment the focused yabai stack's top border; SIGUSR1 requests a refresh.
    #[arg(long)]
    yabai_stack: bool,
}

fn main() -> Result<()> {
    let options = Options::parse();
    ensure!(
        options.width.is_finite() && options.width > 0.0,
        "width must be positive and finite"
    );
    ensure!(
        options.radius.is_finite() && options.radius >= 0.0,
        "radius must be nonnegative and finite"
    );
    ensure!(
        options.scale.is_none_or(|scale| matches!(scale, 1.0 | 2.0)),
        "scale override must be 1 or 2"
    );
    let settings = Settings {
        selected_window: options.window,
        layout: options.layout,
        width: options.width,
        radius: options.radius,
        scale_override: options.scale,
        active_color: options.active_color,
        inactive_color: options.inactive_color,
        verbose: options.verbose,
    };
    let connection = Connection::main();
    let events = Events::new(connection)?;
    let stack_reader = options
        .yabai_stack
        .then(|| StackReader::new(macos::events::wake_main_loop))
        .transpose()?;
    let mut manager = WindowManager::new(connection, settings)?;
    let deadline = options
        .seconds
        .map(|seconds| Instant::now() + Duration::from_secs(seconds));
    loop {
        if events.should_exit() || deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            break;
        }
        let changes = events.take_changes();
        let changed = !changes.is_empty();
        if changed {
            if changes.stack_refresh
                && let Some(reader) = &stack_reader
            {
                reader.request_update()?;
            }
            if !manager.refresh(changes)? {
                break;
            }
        }
        let stack_update = stack_reader.as_ref().and_then(StackReader::take_update);
        let stack_changed = stack_update.is_some();
        if let Some(position) = stack_update {
            manager.update_stack(position?)?;
        }
        if !changed && !stack_changed {
            events.wait(deadline.map_or(f64::MAX, |deadline| {
                deadline
                    .saturating_duration_since(Instant::now())
                    .as_secs_f64()
            }));
        }
    }
    println!("exit updates={}", manager.updates);
    Ok(())
}
