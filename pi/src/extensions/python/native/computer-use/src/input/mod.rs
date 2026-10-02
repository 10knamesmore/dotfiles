//! Track only this worker's virtual input ownership and release scopes in reverse order.

mod keyboard;
mod pointer;

use std::cell::{Cell, RefCell};
use std::sync::OnceLock;
use std::thread::ThreadId;
use std::time::Instant;

use pyo3::prelude::*;
use xkbcommon::xkb;

use crate::wayland::{Desktop, VirtualKeyboard, VirtualPointer};
use crate::{logging, wait};
use keyboard::{Key, Keyboard};

static OWNER: OnceLock<ThreadId> = OnceLock::new();
thread_local! {
    static INPUT: RefCell<Option<Input>> = const { RefCell::new(None) };
    static NEXT_ACQUISITION: Cell<u64> = const { Cell::new(1) };
}

/// Identify a particular acquisition so old scopes cannot release a later re-press.
struct HeldKey {
    /// Validated base key currently owned by this worker.
    key: Key,

    /// Worker-lifetime token that is never reused after close/reconnect.
    acquisition: u64,
}

/// Own lazily created devices for the lifetime of one Python worker connection.
struct Input {
    /// Dedicated connection, independent of short-lived screenshot connections.
    desktop: Desktop,

    /// Named-key keyboard, created only by the first key acquisition.
    keyboard: Option<Keyboard>,

    /// Unicode keyboard whose map can change without changing held named keys.
    text_keyboard: Option<VirtualKeyboard>,

    /// Output-bound pointers by connector name; the empty name is an unbound wheel device.
    pointers: std::collections::HashMap<String, VirtualPointer>,

    /// Held named keys in acquisition order; physical keys never enter this list.
    keys: Vec<HeldKey>,

    /// Keyboard events may still be passing through the input method before pointer input.
    keyboard_pending: bool,

    /// Pointer device and evdev button codes awaiting release during an action.
    buttons: Vec<(VirtualPointer, u32)>,

    /// Monotonic origin for all event timestamps on these devices.
    clock: Instant,
}

pub(crate) fn mark_owner() {
    OWNER.get_or_init(|| std::thread::current().id());
}

fn check_owner() -> PyResult<()> {
    if OWNER
        .get()
        .is_some_and(|owner| *owner != std::thread::current().id())
    {
        return Err(wait::runtime(
            "computer_use input must run on the Python thread that imported it",
        ));
    }
    Ok(())
}

impl Input {
    fn new(py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            desktop: Desktop::connect(py)?,
            keyboard: None,
            text_keyboard: None,
            pointers: Default::default(),
            keys: Vec::new(),
            keyboard_pending: false,
            buttons: Vec::new(),
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

    fn acquire(&mut self, keys: &[Key]) -> PyResult<Vec<u64>> {
        self.ensure_keyboard()?;
        let mut acquisitions = Vec::new();
        for key in keys {
            if self.keys.iter().any(|held| held.key.code == key.code) {
                continue;
            }
            let acquisition = NEXT_ACQUISITION.with(|next| {
                let id = next.get();
                next.set(id + 1);
                id
            });
            let time = self.time();
            self.keyboard.as_mut().unwrap().key(key.code, true, time);
            self.keyboard_pending = true;
            self.keys.push(HeldKey {
                key: key.clone(),
                acquisition,
            });
            acquisitions.push(acquisition);
        }
        Ok(acquisitions)
    }

    fn release_acquisitions(&mut self, acquisitions: &[u64]) {
        for acquisition in acquisitions.iter().rev() {
            if let Some(index) = self
                .keys
                .iter()
                .position(|held| held.acquisition == *acquisition)
            {
                self.release_key(index);
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

    fn release_keys(&mut self) -> PyResult<()> {
        while !self.keys.is_empty() {
            self.release_key(self.keys.len() - 1);
        }
        if let Some(keyboard) = self.keyboard.as_mut() {
            keyboard.reset_modifiers()?;
            self.keyboard_pending = true;
        }
        Ok(())
    }

    fn release_buttons(&mut self) {
        while let Some((pointer, button)) = self.buttons.pop() {
            pointer.button(
                self.time(),
                button,
                wayland_client::protocol::wl_pointer::ButtonState::Released,
            );
            pointer.frame();
        }
    }
}

/// Roll back keys acquired by a failed operation without stealing enclosing scopes.
fn with_input<T>(
    py: Python<'_>,
    operation: &str,
    action: impl FnOnce(&mut Input) -> PyResult<T>,
) -> PyResult<T> {
    check_owner()?;
    let result = INPUT.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| wait::runtime("input operation is already running"))?;
        if slot.is_none() {
            *slot = Some(Input::new(py)?);
        }
        let input = slot.as_mut().unwrap();
        let previous: Vec<_> = input.keys.iter().map(|held| held.acquisition).collect();
        let result = action(input).and_then(|value| {
            input.desktop.sync(py, true)?;
            Ok(value)
        });
        if result.is_err() {
            let acquired: Vec<_> = input
                .keys
                .iter()
                .filter(|held| !previous.contains(&held.acquisition))
                .map(|held| held.acquisition)
                .collect();
            input.release_acquisitions(&acquired);
            input.release_buttons();
            if input.desktop.sync(py, false).is_err() {
                logging::event("input.rollback", "connection-lost");
                *slot = None;
            } else {
                logging::event("input.rollback", "ok");
            }
        }
        result
    });
    logging::result(operation, result)
}

fn with_existing<T>(
    operation: &str,
    absent: T,
    action: impl FnOnce(&mut Input) -> PyResult<T>,
) -> PyResult<T> {
    check_owner()?;
    let result = INPUT.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| wait::runtime("input operation is already running"))?;
        match slot.as_mut() {
            Some(input) => action(input),
            None => Ok(absent),
        }
    });
    logging::result(operation, result)
}

/// Press a chord repeatedly; keys already held by this worker remain held.
#[pyfunction]
#[pyo3(signature = (*keys, count = 1, interval = 0.05))]
fn press(py: Python<'_>, keys: Vec<String>, count: u32, interval: f64) -> PyResult<()> {
    let keys = keyboard::resolve(py, &keys)?;
    let interval = wait::seconds(interval, "interval")?;
    if count == 0 {
        return Err(wait::invalid("count must be greater than zero"));
    }
    with_input(py, "press", |input| {
        for iteration in 0..count {
            py.check_signals()?;
            let acquired = input.acquire(&keys)?;
            input.release_acquisitions(&acquired);
            input.desktop.sync(py, true)?;
            if iteration + 1 < count {
                wait::pause(py, interval)?;
            }
        }
        Ok(())
    })
}

/// Hold validated keys until key_up(), scope cleanup or worker cleanup releases them.
#[pyfunction]
#[pyo3(signature = (*keys))]
fn key_down(py: Python<'_>, keys: Vec<String>) -> PyResult<()> {
    let keys = keyboard::resolve(py, &keys)?;
    with_input(py, "key_down", |input| input.acquire(&keys).map(|_| ()))
}

/// Release only named keys owned by this worker; physical keys are unaffected.
#[pyfunction]
#[pyo3(signature = (*keys))]
fn key_up(py: Python<'_>, keys: Vec<String>) -> PyResult<()> {
    let keys = keyboard::resolve(py, &keys)?;
    with_existing("key_up", (), |input| {
        for key in keys.iter().rev() {
            if let Some(index) = input.keys.iter().position(|held| held.key.code == key.code) {
                input.release_key(index);
            }
        }
        input.desktop.sync(py, false)
    })
}

/// Return canonical XKB names in acquisition order, without opening a connection.
#[pyfunction]
fn held_keys(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let names = with_existing("held_keys", Vec::new(), |input| {
        Ok(input
            .keys
            .iter()
            .map(|held| held.key.name.clone())
            .collect::<Vec<_>>())
    })?;
    Ok(pyo3::types::PyTuple::new(py, names)?.into_any().unbind())
}

/// Release the worker's keys and virtual modifier state without connecting if unused.
#[pyfunction]
fn release_keys(py: Python<'_>) -> PyResult<()> {
    with_existing("release_keys", (), |input| {
        input.release_keys()?;
        input.desktop.sync(py, false)
    })
}

/// Release worker-owned keys and mouse buttons after a failed or cancelled cell.
#[pyfunction]
pub(crate) fn _release_inputs(py: Python<'_>) -> PyResult<()> {
    let result = with_existing("release_inputs", (), |input| {
        input.release_buttons();
        input.release_keys()?;
        input.desktop.sync(py, false)
    });
    if result.is_err() {
        INPUT.with(|slot| {
            if let Ok(mut slot) = slot.try_borrow_mut() {
                *slot = None;
            }
        });
    }
    result
}

/// Release all owned input and close connections; later input calls may reconnect.
#[pyfunction]
pub(crate) fn close(py: Python<'_>) -> PyResult<()> {
    check_owner()?;
    let result = _release_inputs(py);
    INPUT.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| wait::runtime("input operation is already running"))?;
        if let Some(input) = slot.take() {
            if let Some(keyboard) = &input.keyboard {
                keyboard.device.destroy();
            }
            if let Some(keyboard) = &input.text_keyboard {
                keyboard.destroy();
            }
            for pointer in input.pointers.values() {
                pointer.destroy();
            }
            let _ = input.desktop.connection.flush();
        }
        Ok::<_, PyErr>(())
    })?;
    logging::result("close", result)
}

/// A reusable scope whose acquisitions are distinct from outer or later holds.
#[pyclass(module = "computer_use")]
struct Hold {
    /// Fully validated chord; construction does not create a device or press keys.
    keys: Vec<Key>,

    /// Tokens acquired by the current entry; None means this scope is not entered.
    acquisitions: Option<Vec<u64>>,
}

#[pymethods]
impl Hold {
    fn __enter__(&mut self, py: Python<'_>) -> PyResult<()> {
        if self.acquisitions.is_some() {
            return Err(wait::runtime("this hold scope is already entered"));
        }
        self.acquisitions = Some(with_input(py, "hold.enter", |input| {
            input.acquire(&self.keys)
        })?);
        Ok(())
    }

    fn __exit__(
        &mut self,
        py: Python<'_>,
        exc_type: &Bound<'_, PyAny>,
        _exc_value: &Bound<'_, PyAny>,
        _traceback: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        if let Some(acquisitions) = self.acquisitions.take() {
            let result = with_existing("hold.exit", (), |input| {
                input.release_acquisitions(&acquisitions);
                input.desktop.sync(py, false)
            });
            if exc_type.is_none() {
                result?;
            }
        }
        Ok(false)
    }
}

/// Validate a chord now and acquire only its unheld keys on entering the scope.
#[pyfunction]
#[pyo3(signature = (*keys))]
fn hold(py: Python<'_>, keys: Vec<String>) -> PyResult<Hold> {
    Ok(Hold {
        keys: keyboard::resolve(py, &keys)?,
        acquisitions: None,
    })
}

/// Type Unicode without the clipboard; newline and tab produce their respective keys.
#[pyfunction]
#[pyo3(signature = (text, /, *, interval = 0.0))]
fn type_text(py: Python<'_>, text: &str, interval: f64) -> PyResult<()> {
    let interval = wait::seconds(interval, "interval")?;
    let chunks = keyboard::text_chunks(py, text)?;
    if chunks.is_empty() {
        return Ok(());
    }
    with_input(py, "type_text", |input| {
        if !input.keys.is_empty() {
            return Err(wait::invalid(
                "release this worker's held keys before type_text; use press for chords",
            ));
        }
        if input.text_keyboard.is_none() {
            input.text_keyboard = Some(input.create_keyboard()?);
            logging::event("text_keyboard.create", "ok");
        }
        let device = input.text_keyboard.as_ref().unwrap().clone();
        let mut first = true;
        for chunk in chunks {
            keyboard::upload(&device, &chunk.map)?;
            device.modifiers(0, 0, 0, 0);
            input.desktop.sync(py, true)?;
            for code in chunk.codes {
                if !first {
                    wait::pause(py, interval)?;
                }
                first = false;
                py.check_signals()?;
                // No Python call or fallible operation separates down from up.
                device.key(input.time(), code, 1);
                device.key(input.time(), code, 0);
                input.keyboard_pending = true;
                input.desktop.sync(py, true)?;
            }
        }
        Ok(())
    })
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(press, module)?)?;
    module.add_function(wrap_pyfunction!(key_down, module)?)?;
    module.add_function(wrap_pyfunction!(key_up, module)?)?;
    module.add_function(wrap_pyfunction!(held_keys, module)?)?;
    module.add_function(wrap_pyfunction!(release_keys, module)?)?;
    module.add_function(wrap_pyfunction!(_release_inputs, module)?)?;
    module.add_function(wrap_pyfunction!(close, module)?)?;
    module.add_function(wrap_pyfunction!(hold, module)?)?;
    module.add_function(wrap_pyfunction!(type_text, module)?)?;
    pointer::register(module)
}
