//! Own virtual devices and acquisition tokens on one desktop's execution thread.

mod keyboard;
mod pointer;

use pyo3::prelude::*;
use serde_json::json;
use std::time::{Duration, Instant};
use wayland_client::protocol::wl_pointer::ButtonState;
use xkbcommon::xkb;

use crate::desktop::{Control, Target};
use crate::wayland::{Desktop, VirtualKeyboard, VirtualPointer};
use crate::{logging, wait};
use keyboard::{Key, Keyboard};

struct HeldKey {
    key: Key,
    /// Never reused, so an old scope cannot release a later acquisition.
    acquisition: u64,
}

struct HeldButton {
    pointer: VirtualPointer,
    button: u32,
    /// Never reused, so an old scope cannot release a later acquisition.
    acquisition: u64,
}

/// XKB and Wayland objects never leave the thread that constructs this value.
pub(crate) struct Input {
    desktop: Desktop,
    keyboard: Option<Keyboard>,
    text_keyboard: Option<VirtualKeyboard>,
    pointers: std::collections::HashMap<String, VirtualPointer>,
    keys: Vec<HeldKey>,
    keyboard_pending: bool,
    buttons: Vec<HeldButton>,
    next_acquisition: u64,
    clock: Instant,
}

impl Input {
    pub(crate) fn new(control: &Control, target: &Target) -> PyResult<Self> {
        Ok(Self {
            desktop: Desktop::connect(control, target)?,
            keyboard: None,
            text_keyboard: None,
            pointers: Default::default(),
            keys: Vec::new(),
            keyboard_pending: false,
            buttons: Vec::new(),
            next_acquisition: 1,
            clock: Instant::now(),
        })
    }

    fn time(&self) -> u32 {
        self.clock.elapsed().as_millis() as u32
    }

    fn create_keyboard(&self) -> PyResult<VirtualKeyboard> {
        let manager = self
            .desktop
            .state
            .keyboard_manager
            .as_ref()
            .ok_or_else(|| wait::runtime("zwp_virtual_keyboard_manager_v1 is required"))?;
        let seat = self
            .desktop
            .state
            .seat
            .as_ref()
            .ok_or_else(|| wait::runtime("Wayland seat was not found"))?;
        Ok(manager.create_virtual_keyboard(seat, &self.desktop.queue.handle(), ()))
    }

    fn ensure_keyboard(&mut self) -> PyResult<()> {
        if self.keyboard.is_none() {
            let map = keyboard::keymap()?;
            let device = self.create_keyboard()?;
            if let Err(error) = keyboard::upload(&device, &map) {
                device.destroy();
                return Err(error);
            }
            self.keyboard = Some(Keyboard {
                device,
                state: xkb::State::new(&map),
            });
            logging::event("keyboard.create", "ok");
        }
        Ok(())
    }

    fn acquire(&mut self, control: &Control, keys: &[Key]) -> PyResult<Vec<u64>> {
        control.check()?;
        self.ensure_keyboard()?;
        let mut acquisitions = Vec::new();
        for key in keys {
            if self.keys.iter().any(|held| held.key.code == key.code) {
                continue;
            }
            let acquisition = self.next_acquisition;
            self.next_acquisition += 1;
            let time = self.time();
            self.keyboard.as_mut().unwrap().key(key.code, true, time);
            self.keyboard_pending = true;
            self.keys.push(HeldKey {
                key: key.clone(),
                acquisition,
            });
            acquisitions.push(acquisition);
        }
        self.show_keys(control)?;
        Ok(acquisitions)
    }

    /// Release the given acquisitions on keys and buttons, newest first.
    pub(crate) fn release_acquisitions(&mut self, control: &Control, acquisitions: &[u64]) {
        for acquisition in acquisitions.iter().rev() {
            if let Some(index) = self
                .keys
                .iter()
                .position(|held| held.acquisition == *acquisition)
            {
                self.release_key(index);
            } else if let Some(index) = self
                .buttons
                .iter()
                .position(|held| held.acquisition == *acquisition)
            {
                self.release_button_at(control, index);
            }
        }
    }

    fn release_key(&mut self, index: usize) {
        let held = self.keys.remove(index);
        let time = self.time();
        if let Some(keyboard) = self.keyboard.as_mut() {
            keyboard.key(held.key.code, false, time);
            self.keyboard_pending = true;
        }
    }

    pub(crate) fn held_keys(&self) -> Vec<String> {
        self.keys.iter().map(|held| held.key.name.clone()).collect()
    }

    fn show_keys(&self, control: &Control) -> PyResult<()> {
        control.event(json!({"type":"keys", "keys":self.held_keys()}))
    }

    pub(crate) fn release_keys(&mut self) -> PyResult<()> {
        while !self.keys.is_empty() {
            self.release_key(self.keys.len() - 1);
        }
        if let Some(keyboard) = self.keyboard.as_mut() {
            keyboard.reset_modifiers()?;
            self.keyboard_pending = true;
        }
        Ok(())
    }

    fn release_all_buttons(&mut self, control: &Control) {
        while !self.buttons.is_empty() {
            self.release_button_at(control, self.buttons.len() - 1);
        }
    }

    /// Press one button unless this desktop already holds it; returns the acquisition token.
    fn acquire_button(
        &mut self,
        control: &Control,
        pointer: &VirtualPointer,
        button: u32,
    ) -> PyResult<Option<u64>> {
        if self.buttons.iter().any(|held| held.button == button) {
            return Ok(None);
        }
        control.check()?;
        pointer.button(self.time(), button, ButtonState::Pressed);
        pointer.frame();
        let acquisition = self.next_acquisition;
        self.next_acquisition += 1;
        self.buttons.push(HeldButton {
            pointer: pointer.clone(),
            button,
            acquisition,
        });
        control.event(
            json!({"type":"button", "button":pointer::button_name(button), "pressed":true}),
        )?;
        Ok(Some(acquisition))
    }

    fn release_button_at(&mut self, control: &Control, index: usize) {
        let held = self.buttons.remove(index);
        held.pointer
            .button(self.time(), held.button, ButtonState::Released);
        held.pointer.frame();
        // A failed display channel must never prevent the matching Wayland button release.
        let _ = control.event(
            json!({"type":"button", "button":pointer::button_name(held.button), "pressed":false}),
        );
    }

    fn release_button_code(&mut self, control: &Control, button: u32) {
        while let Some(index) = self.buttons.iter().position(|held| held.button == button) {
            self.release_button_at(control, index);
        }
    }

    /// Always attempt the Wayland releases, even after cancel, revoke or prompt EOF.
    pub(crate) fn release_all(&mut self, control: &Control) -> PyResult<()> {
        self.release_all_buttons(control);
        let keys = self.release_keys();
        let sync = self.desktop.sync(control, false);
        let result = keys.and(sync);
        if result.is_err() {
            control.disconnect();
        }
        logging::result("input.cleanup", result)
    }

    pub(crate) fn run<T>(
        &mut self,
        control: &Control,
        action: impl FnOnce(&mut Self) -> PyResult<T>,
    ) -> PyResult<T> {
        let previous: Vec<u64> = self
            .keys
            .iter()
            .map(|held| held.acquisition)
            .chain(self.buttons.iter().map(|held| held.acquisition))
            .collect();
        let result = action(self).and_then(|value| {
            if let Err(error) = self.desktop.sync(control, true) {
                control.disconnect();
                return Err(error);
            }
            Ok(value)
        });
        if result.is_err() {
            let acquired: Vec<_> = self
                .keys
                .iter()
                .map(|held| held.acquisition)
                .chain(self.buttons.iter().map(|held| held.acquisition))
                .filter(|acquisition| !previous.contains(acquisition))
                .collect();
            self.release_acquisitions(control, &acquired);
            if self.desktop.sync(control, false).is_err() {
                control.disconnect();
                logging::event("input.rollback", "connection-lost");
            } else {
                logging::event("input.rollback", "ok");
            }
        }
        let shown = self.show_keys(control);
        result.and_then(|value| shown.map(|_| value))
    }

    pub(crate) fn press(
        &mut self,
        control: &Control,
        names: &[String],
        count: u32,
        interval: f64,
    ) -> PyResult<()> {
        let keys = keyboard::resolve(control, names)?;
        let interval = wait::seconds(interval, "interval")?;
        if count == 0 {
            return Err(wait::invalid("count must be greater than zero"));
        }
        self.run(control, |input| {
            for iteration in 0..count {
                control.check()?;
                let acquired = input.acquire(control, &keys)?;
                input.release_acquisitions(control, &acquired);
                input.show_keys(control)?;
                input.desktop.sync(control, true)?;
                if iteration + 1 < count {
                    wait::pause(control, interval)?;
                }
            }
            Ok(())
        })
    }

    pub(crate) fn key_down(&mut self, control: &Control, names: &[String]) -> PyResult<Vec<u64>> {
        let keys = keyboard::resolve(control, names)?;
        self.run(control, |input| input.acquire(control, &keys))
    }

    pub(crate) fn validate_keys(control: &Control, names: &[String]) -> PyResult<()> {
        keyboard::resolve(control, names).map(|_| ())
    }

    fn resolve_buttons(names: &[String]) -> PyResult<Vec<u32>> {
        names.iter().map(|name| pointer::button(name)).collect()
    }

    pub(crate) fn button_down(
        &mut self,
        control: &Control,
        names: &[String],
    ) -> PyResult<Vec<u64>> {
        let buttons = Self::resolve_buttons(names)?;
        self.run(control, |input| {
            let pointer = input.pointer(None)?;
            let mut acquisitions = Vec::new();
            for button in buttons {
                if let Some(acquisition) = input.acquire_button(control, &pointer, button)? {
                    acquisitions.push(acquisition);
                }
            }
            Ok(acquisitions)
        })
    }

    pub(crate) fn button_up(&mut self, control: &Control, names: &[String]) -> PyResult<()> {
        let buttons = Self::resolve_buttons(names)?;
        self.run(control, |input| {
            for button in buttons {
                input.release_button_code(control, button);
            }
            Ok(())
        })
    }

    pub(crate) fn release_buttons(&mut self, control: &Control) -> PyResult<()> {
        self.run(control, |input| {
            input.release_all_buttons(control);
            Ok(())
        })
    }

    pub(crate) fn key_up(&mut self, control: &Control, names: &[String]) -> PyResult<()> {
        let keys = keyboard::resolve(control, names)?;
        self.run(control, |input| {
            for key in keys.iter().rev() {
                if let Some(index) = input.keys.iter().position(|held| held.key.code == key.code) {
                    input.release_key(index);
                }
            }
            Ok(())
        })
    }

    pub(crate) fn type_text(
        &mut self,
        control: &Control,
        text: &str,
        interval: f64,
    ) -> PyResult<()> {
        let interval = wait::seconds(interval, "interval")?;
        let chunks = keyboard::text_chunks(control, text)?;
        self.run(control, |input| {
            if !input.keys.is_empty() {
                return Err(wait::invalid(
                    "release this desktop's held keys before type_text; use press for chords",
                ));
            }
            control.event(json!({"type":"text", "characters":text.chars().count()}))?;
            if chunks.is_empty() {
                return Ok(());
            }
            // Named shortcuts must reach the client before their keymap is replaced with text.
            input.settle_keyboard(control)?;
            if input.text_keyboard.is_none() {
                input.text_keyboard = Some(input.create_keyboard()?);
                logging::event("text_keyboard.create", "ok");
            }
            let device = input.text_keyboard.as_ref().unwrap().clone();
            let mut first = true;
            for chunk in chunks {
                keyboard::upload(&device, &chunk.map)?;
                device.modifiers(0, 0, 0, 0);
                input.desktop.sync(control, true)?;
                for code in chunk.codes {
                    if !first {
                        wait::pause(control, interval)?;
                    }
                    first = false;
                    control.check()?;
                    // No cancellation or fallible call separates down from up.
                    device.key(input.time(), code, 1);
                    device.key(input.time(), code, 0);
                    input.keyboard_pending = true;
                    input.desktop.sync(control, true)?;
                    // A compositor roundtrip does not mean X11 clients consumed these keys.
                    // Give them time before the next chunk or call replaces the keymap.
                    wait::pause(control, Duration::from_millis(4))?;
                }
            }
            Ok(())
        })
    }

    fn settle_keyboard(&mut self, control: &Control) -> PyResult<()> {
        if self.keyboard_pending {
            // Wayland acknowledges compositor receipt, not client or input-method processing.
            self.desktop.sync(control, true)?;
            wait::pause(control, Duration::from_millis(20))?;
            self.keyboard_pending = false;
        }
        Ok(())
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        if let Some(keyboard) = &self.keyboard {
            keyboard.device.destroy();
        }
        if let Some(keyboard) = &self.text_keyboard {
            keyboard.destroy();
        }
        for pointer in self.pointers.values() {
            pointer.destroy();
        }
        let _ = self.desktop.connection.flush();
    }
}
