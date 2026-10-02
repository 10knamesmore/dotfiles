//! Map capture image pixels to monitor-local pointer events and discrete wheel notches.

use std::time::{Duration, Instant};

use pyo3::prelude::*;
use wayland_client::protocol::wl_pointer::{Axis, AxisSource, ButtonState};

use super::{Input, with_input};
use crate::capture::Capture;
use crate::hyprland::{self, Monitor};
use crate::wait;
use crate::wayland::VirtualPointer;

fn point(x: f64, y: f64, relative_to: Option<&Capture>) -> PyResult<(f64, f64)> {
    match relative_to {
        Some(shot) => shot.desktop_point(x, y),
        None if x.is_finite() && y.is_finite() => Ok((x, y)),
        None => Err(wait::invalid("coordinates must be finite")),
    }
}

fn monitor_at(monitors: &[Monitor], point: (f64, f64)) -> PyResult<&Monitor> {
    monitors
        .iter()
        .find(|monitor| {
            let (x, y, width, height) = monitor.bounds();
            point.0 >= x && point.1 >= y && point.0 < x + width && point.1 < y + height
        })
        .ok_or_else(|| wait::invalid("coordinates are outside the active monitors"))
}

fn button(name: &str) -> PyResult<u32> {
    match name {
        "left" => Ok(0x110),
        "right" => Ok(0x111),
        "middle" => Ok(0x112),
        "back" => Ok(0x116),
        "forward" => Ok(0x115),
        _ => Err(wait::invalid(
            "button must be left, right, middle, back or forward",
        )),
    }
}

impl Input {
    fn settle_keyboard(&mut self, py: Python<'_>) -> PyResult<()> {
        if self.keyboard_pending {
            // Hyprland can forward keys through an input method while delivering
            // pointer events directly. A sync only acknowledges compositor receipt;
            // space the switch so pointer events do not overtake modifier changes.
            self.desktop.sync(py, true)?;
            wait::pause(py, Duration::from_millis(20))?;
            self.keyboard_pending = false;
        }
        Ok(())
    }

    fn pointer(&mut self, monitor: Option<&Monitor>) -> PyResult<VirtualPointer> {
        let name = monitor.map_or("", |monitor| monitor.name.as_str());
        if let Some(pointer) = self.pointers.get(name) {
            return Ok(pointer.clone());
        }
        let output = if monitor.is_some() {
            Some(
                &self
                    .desktop
                    .state
                    .outputs
                    .values()
                    .find(|output| output.name == name)
                    .ok_or_else(|| wait::runtime("monitor is no longer advertised by Wayland"))?
                    .proxy,
            )
        } else {
            None
        };
        let manager = self
            .desktop
            .state
            .pointer_manager
            .as_ref()
            .ok_or_else(|| wait::runtime("zwlr_virtual_pointer_manager_v1 v2 is required"))?;
        let pointer = manager.create_virtual_pointer_with_output(
            self.desktop.state.seat.as_ref(),
            output,
            &self.desktop.queue.handle(),
            (),
        );
        self.pointers.insert(name.to_string(), pointer.clone());
        crate::logging::event("pointer.create", "ok");
        Ok(pointer)
    }

    fn move_to(&mut self, monitors: &[Monitor], point: (f64, f64)) -> PyResult<VirtualPointer> {
        let monitor = monitor_at(monitors, point)?;
        let pointer = self.pointer(Some(monitor))?;
        let (x, y, width, height) = monitor.bounds();
        // An output-bound pointer avoids relying on compositor-wide normalization,
        // whose origin differs from desktop coordinates when monitors start negative.
        let extent = 1_000_000;
        pointer.motion_absolute(
            self.time(),
            ((point.0 - x) / width * extent as f64).round() as u32,
            ((point.1 - y) / height * extent as f64).round() as u32,
            extent,
            extent,
        );
        pointer.frame();
        Ok(pointer)
    }

    fn button_down(&mut self, pointer: &VirtualPointer, button: u32) {
        pointer.button(self.time(), button, ButtonState::Pressed);
        pointer.frame();
        self.buttons.push((pointer.clone(), button));
    }
}

/// Move in desktop logical coordinates or in the returned pixels of relative_to.
#[pyfunction]
#[pyo3(signature = (x, y, *, relative_to = None))]
fn move_pointer(
    py: Python<'_>,
    x: f64,
    y: f64,
    relative_to: Option<PyRef<'_, Capture>>,
) -> PyResult<()> {
    let point = point(x, y, relative_to.as_deref())?;
    let monitors = hyprland::monitors(py)?;
    monitor_at(&monitors, point)?;
    with_input(py, "move_pointer", |input| {
        input.settle_keyboard(py)?;
        input.move_to(&monitors, point).map(|_| ())
    })
}

/// Move then click; count repeats the complete button down/up pair.
#[pyfunction]
#[pyo3(signature = (x, y, *, button = "left", count = 1, relative_to = None))]
fn click(
    py: Python<'_>,
    x: f64,
    y: f64,
    button: &str,
    count: u32,
    relative_to: Option<PyRef<'_, Capture>>,
) -> PyResult<()> {
    let button = self::button(button)?;
    if count == 0 {
        return Err(wait::invalid("count must be greater than zero"));
    }
    let point = point(x, y, relative_to.as_deref())?;
    let monitors = hyprland::monitors(py)?;
    monitor_at(&monitors, point)?;
    with_input(py, "click", |input| {
        input.settle_keyboard(py)?;
        let pointer = input.move_to(&monitors, point)?;
        for iteration in 0..count {
            py.check_signals()?;
            input.button_down(&pointer, button);
            input.release_buttons();
            input.desktop.sync(py, true)?;
            if iteration + 1 < count {
                wait::pause(py, Duration::from_millis(50))?;
            }
        }
        Ok(())
    })
}

/// Hold a button while interpolating a line between two desktop or capture image points.
#[pyfunction]
#[pyo3(signature = (start, end, *, button = "left", duration = 0.3, relative_to = None))]
fn drag(
    py: Python<'_>,
    start: (f64, f64),
    end: (f64, f64),
    button: &str,
    duration: f64,
    relative_to: Option<PyRef<'_, Capture>>,
) -> PyResult<()> {
    let button = self::button(button)?;
    let duration = wait::seconds(duration, "duration")?;
    let start = point(start.0, start.1, relative_to.as_deref())?;
    let end = point(end.0, end.1, relative_to.as_deref())?;
    let monitors = hyprland::monitors(py)?;
    monitor_at(&monitors, start)?;
    monitor_at(&monitors, end)?;
    with_input(py, "drag", |input| {
        input.settle_keyboard(py)?;
        let pointer = input.move_to(&monitors, start)?;
        input.button_down(&pointer, button);
        input.desktop.sync(py, true)?;
        let began = Instant::now();
        loop {
            py.check_signals()?;
            let fraction = if duration.is_zero() {
                1.0
            } else {
                (began.elapsed().as_secs_f64() / duration.as_secs_f64()).min(1.0)
            };
            input.move_to(
                &monitors,
                (
                    start.0 + (end.0 - start.0) * fraction,
                    start.1 + (end.1 - start.1) * fraction,
                ),
            )?;
            input.desktop.sync(py, true)?;
            if fraction >= 1.0 {
                break;
            }
            wait::pause(
                py,
                Duration::from_millis(10).min(duration.saturating_sub(began.elapsed())),
            )?;
        }
        input.release_buttons();
        Ok(())
    })
}

/// Send discrete notches at the current pointer; positive means down or right.
#[pyfunction]
#[pyo3(signature = (amount, *, horizontal = false))]
fn scroll(py: Python<'_>, amount: i32, horizontal: bool) -> PyResult<()> {
    if amount == 0 {
        return Ok(());
    }
    with_input(py, "scroll", |input| {
        input.settle_keyboard(py)?;
        let pointer = input.pointer(None)?;
        let axis = if horizontal {
            Axis::HorizontalScroll
        } else {
            Axis::VerticalScroll
        };
        for _ in 0..amount.unsigned_abs() {
            py.check_signals()?;
            pointer.axis_discrete(
                input.time(),
                axis,
                amount.signum() as f64 * 15.0,
                amount.signum(),
            );
            pointer.axis_source(AxisSource::Wheel);
            pointer.frame();
            input.desktop.sync(py, true)?;
        }
        Ok(())
    })
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(move_pointer, module)?)?;
    module.add_function(wrap_pyfunction!(click, module)?)?;
    module.add_function(wrap_pyfunction!(drag, module)?)?;
    module.add_function(wrap_pyfunction!(scroll, module)?)?;
    Ok(())
}
