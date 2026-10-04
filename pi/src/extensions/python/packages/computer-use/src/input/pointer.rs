//! Map desktop coordinates to output-local virtual pointer events and prompt events.

use pyo3::prelude::*;
use serde_json::json;
use std::time::{Duration, Instant};
use wayland_client::protocol::wl_pointer::{Axis, AxisSource, ButtonState};

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

    fn button_down(
        &mut self,
        control: &Control,
        pointer: &VirtualPointer,
        button: u32,
    ) -> PyResult<()> {
        control.check()?;
        pointer.button(self.time(), button, ButtonState::Pressed);
        pointer.frame();
        self.buttons.push((pointer.clone(), button));
        control.event(json!({"type":"button", "button":button_name(button), "pressed":true}))
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
            input.move_to(control, &monitors, point).map(|_| ())
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
                input.button_down(control, &pointer, button)?;
                input.release_buttons(control);
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
            input.button_down(control, &pointer, button)?;
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
            input.release_buttons(control);
            Ok(())
        })
    }

    pub(crate) fn scroll(
        &mut self,
        control: &Control,
        amount: i32,
        horizontal: bool,
    ) -> PyResult<()> {
        self.run(control, |input| {
            input.settle_keyboard(control)?;
            let pointer = input.pointer(None)?;
            let axis = if horizontal {
                Axis::HorizontalScroll
            } else {
                Axis::VerticalScroll
            };
            control.event(json!({"type":"scroll", "amount":amount, "horizontal":horizontal}))?;
            for _ in 0..amount.unsigned_abs() {
                control.check()?;
                pointer.axis_discrete(
                    input.time(),
                    axis,
                    amount.signum() as f64 * 15.0,
                    amount.signum(),
                );
                pointer.axis_source(AxisSource::Wheel);
                pointer.frame();
                input.desktop.sync(control, true)?;
            }
            Ok(())
        })
    }
}
