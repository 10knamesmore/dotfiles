//! Map desktop coordinates to output-local virtual pointer events and prompt events.

use pyo3::prelude::*;
use serde_json::json;
use std::time::{Duration, Instant};
use wayland_client::protocol::wl_pointer::{Axis, AxisSource};

use super::Input;
use crate::desktop::{Control, Target};
use crate::hyprland::{self, Monitor};
use crate::wait;
use crate::wayland::VirtualPointer;

fn monitor_at(monitors: &[Monitor], point: (f64, f64)) -> PyResult<&Monitor> {
    monitors
        .iter()
        .find(|monitor| {
            let (x, y, width, height) = monitor.bounds();
            point.0 >= x && point.1 >= y && point.0 < x + width && point.1 < y + height
        })
        .ok_or_else(|| wait::invalid("coordinates are outside the active monitors"))
}

pub(super) fn button(name: &str) -> PyResult<u32> {
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

pub(super) fn button_name(code: u32) -> &'static str {
    match code {
        0x110 => "left",
        0x111 => "right",
        0x112 => "middle",
        0x116 => "back",
        _ => "forward",
    }
}

impl Input {
    pub(super) fn pointer(&mut self, monitor: Option<&Monitor>) -> PyResult<VirtualPointer> {
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

    fn move_to(
        &mut self,
        control: &Control,
        monitors: &[Monitor],
        point: (f64, f64),
    ) -> PyResult<VirtualPointer> {
        control.check()?;
        let monitor = monitor_at(monitors, point)?;
        let pointer = self.pointer(Some(monitor))?;
        let (x, y, width, height) = monitor.bounds();
        let extent = 1_000_000;
        pointer.motion_absolute(
            self.time(),
            ((point.0 - x) / width * extent as f64).round() as u32,
            ((point.1 - y) / height * extent as f64).round() as u32,
            extent,
            extent,
        );
        pointer.frame();
        control.event(
            json!({"type":"pointer", "monitor":monitor.name, "x":point.0-x, "y":point.1-y}),
        )?;
        Ok(pointer)
    }

    pub(crate) fn move_pointer(
        &mut self,
        control: &Control,
        target: &Target,
        point: (f64, f64),
    ) -> PyResult<()> {
        let monitors = hyprland::monitors(control, target)?;
        monitor_at(&monitors, point)?;
        self.run(control, |input| {
            input.settle_keyboard(control)?;
            // A held button and the absolute pointer are different virtual devices, which clients
            // treat as unrelated input; move the held device's cursor instead. Relative motion
            // also keeps a drag alive across monitors.
            if input.buttons.is_empty() {
                input.move_to(control, &monitors, point).map(|_| ())
            } else {
                let current = hyprland::cursor_position(control, target)?;
                input.move_relative(control, target, (point.0 - current.0, point.1 - current.1))
            }
        })
    }

    pub(crate) fn click(
        &mut self,
        control: &Control,
        target: &Target,
        point: (f64, f64),
        name: &str,
        count: u32,
    ) -> PyResult<()> {
        let button = button(name)?;
        if count == 0 {
            return Err(wait::invalid("count must be greater than zero"));
        }
        let monitors = hyprland::monitors(control, target)?;
        monitor_at(&monitors, point)?;
        self.run(control, |input| {
            input.settle_keyboard(control)?;
            let pointer = input.move_to(control, &monitors, point)?;
            for iteration in 0..count {
                // Release only this click's press; a button held by the caller stays down.
                let acquisitions: Vec<u64> = input
                    .acquire_button(control, &pointer, button)?
                    .into_iter()
                    .collect();
                input.release_acquisitions(control, &acquisitions);
                input.desktop.sync(control, true)?;
                if iteration + 1 < count {
                    wait::pause(control, Duration::from_millis(50))?;
                }
            }
            Ok(())
        })
    }

    pub(crate) fn drag(
        &mut self,
        control: &Control,
        target: &Target,
        start: (f64, f64),
        end: (f64, f64),
        name: &str,
        duration: f64,
    ) -> PyResult<()> {
        let button = button(name)?;
        let duration = wait::seconds(duration, "duration")?;
        let monitors = hyprland::monitors(control, target)?;
        monitor_at(&monitors, start)?;
        monitor_at(&monitors, end)?;
        self.run(control, |input| {
            input.settle_keyboard(control)?;
            let pointer = input.move_to(control, &monitors, start)?;
            let acquisitions: Vec<u64> = input
                .acquire_button(control, &pointer, button)?
                .into_iter()
                .collect();
            input.desktop.sync(control, true)?;
            let began = Instant::now();
            loop {
                control.check()?;
                let fraction = if duration.is_zero() {
                    1.0
                } else {
                    (began.elapsed().as_secs_f64() / duration.as_secs_f64()).min(1.0)
                };
                input.move_to(
                    control,
                    &monitors,
                    (
                        start.0 + (end.0 - start.0) * fraction,
                        start.1 + (end.1 - start.1) * fraction,
                    ),
                )?;
                input.desktop.sync(control, true)?;
                if fraction >= 1.0 {
                    break;
                }
                wait::pause(
                    control,
                    Duration::from_millis(10).min(duration.saturating_sub(began.elapsed())),
                )?;
            }
            input.release_acquisitions(control, &acquisitions);
            Ok(())
        })
    }

    pub(crate) fn move_pointer_relative(
        &mut self,
        control: &Control,
        target: &Target,
        delta: (f64, f64),
    ) -> PyResult<()> {
        if !delta.0.is_finite() || !delta.1.is_finite() {
            return Err(wait::invalid("coordinates must be finite"));
        }
        self.run(control, |input| {
            input.settle_keyboard(control)?;
            input.move_relative(control, target, delta)
        })
    }

    /// Move the cursor by a relative delta on the seat-wide pointer and report the result.
    fn move_relative(
        &mut self,
        control: &Control,
        target: &Target,
        delta: (f64, f64),
    ) -> PyResult<()> {
        let pointer = self.pointer(None)?;
        control.check()?;
        pointer.motion(self.time(), delta.0, delta.1);
        pointer.frame();
        // The overlay cursor follows the real position, which only advances after the compositor applies motion.
        self.desktop.sync(control, true)?;
        let position = hyprland::cursor_position(control, target)?;
        let monitors = hyprland::monitors(control, target)?;
        let monitor = monitor_at(&monitors, position)?;
        let (x, y, _, _) = monitor.bounds();
        control.event(
            json!({"type":"pointer", "monitor":monitor.name, "x":position.0 - x, "y":position.1 - y}),
        )
    }

    pub(crate) fn scroll(
        &mut self,
        control: &Control,
        amount: f64,
        horizontal: bool,
        continuous: bool,
    ) -> PyResult<()> {
        if !amount.is_finite() {
            return Err(wait::invalid("amount must be finite"));
        }
        if !continuous && amount.fract() != 0.0 {
            return Err(wait::invalid(
                "amount must be a whole number; use continuous=True for fractional scroll",
            ));
        }
        if !continuous && (amount > i32::MAX as f64 || amount < i32::MIN as f64) {
            return Err(wait::invalid("amount is out of range"));
        }
        self.run(control, |input| {
            input.settle_keyboard(control)?;
            let pointer = input.pointer(None)?;
            let axis = if horizontal {
                Axis::HorizontalScroll
            } else {
                Axis::VerticalScroll
            };
            control.event(json!({"type":"scroll", "amount":amount, "horizontal":horizontal}))?;
            if continuous {
                if amount != 0.0 {
                    control.check()?;
                    // One whole amount equals the 15 surface units used by axis_discrete.
                    pointer.axis(input.time(), axis, amount * 15.0);
                    pointer.axis_source(AxisSource::Wheel);
                    pointer.frame();
                    input.desktop.sync(control, true)?;
                }
                return Ok(());
            }
            let steps = amount.abs() as i32;
            let direction = amount.signum() as i32;
            for _ in 0..steps {
                control.check()?;
                pointer.axis_discrete(input.time(), axis, direction as f64 * 15.0, direction);
                pointer.axis_source(AxisSource::Wheel);
                pointer.frame();
                input.desktop.sync(control, true)?;
            }
            Ok(())
        })
    }
}
