//! Send page keyboard chords and release every pressed key even if Python cancels the call.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, InsertTextParams,
};
use chromiumoxide::keys::{KeyDefinition, USKEYBOARD_LAYOUT, get_key_definition};
use pyo3::prelude::*;
use tokio::sync::{Mutex, OwnedMutexGuard};

use super::Page;
use crate::{diagnostics, runtime};

#[derive(Clone)]
pub(super) struct Key {
    definition: &'static KeyDefinition,
    modifier: i64,
}

pub(super) fn chord(keys: Vec<String>) -> PyResult<Vec<Key>> {
    parse(keys).map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))
}

fn parse(keys: Vec<String>) -> anyhow::Result<Vec<Key>> {
    if keys.is_empty() {
        bail!("press requires at least one key");
    }
    let mut result = Vec::new();
    let mut character = None;
    for key in keys {
        let lower = key.to_ascii_lowercase();
        let modifier = match lower.as_str() {
            "alt" => Some(("Alt", 1)),
            "ctrl" | "control" => Some(("Control", 2)),
            "super" | "meta" | "cmd" => Some(("Meta", 4)),
            "shift" => Some(("Shift", 8)),
            _ => None,
        };
        if let Some((name, modifier)) = modifier {
            if result.iter().any(|key: &Key| key.modifier == modifier) {
                bail!("Duplicate modifier: {key}");
            }
            result.push(Key {
                definition: get_key_definition(name).context("Unknown modifier")?,
                modifier,
            });
        } else {
            if character.is_some() {
                bail!("A chord takes modifiers and at most one non-modifier key");
            }
            let name = match lower.as_str() {
                "enter" | "return" => "Enter",
                "esc" | "escape" => "Escape",
                "space" => " ",
                "left" => "ArrowLeft",
                "right" => "ArrowRight",
                "up" => "ArrowUp",
                "down" => "ArrowDown",
                _ => &lower,
            };
            let definition = get_key_definition(name)
                .or_else(|| {
                    USKEYBOARD_LAYOUT
                        .iter()
                        .find(|definition| definition.key.eq_ignore_ascii_case(name))
                })
                .with_context(|| {
                    format!("Unknown key: {key}; pass modifiers as separate arguments")
                })?;
            character = Some(definition);
        }
    }
    if let Some(mut definition) = character {
        if result.iter().any(|key| key.modifier == 8)
            && definition.key.chars().count() == 1
            && let Some(shifted) = USKEYBOARD_LAYOUT.iter().find(|candidate| {
                candidate.code == definition.code
                    && candidate.key != definition.key
                    && candidate.key.chars().count() == 1
            })
        {
            definition = shifted;
        }
        result.push(Key {
            definition,
            modifier: 0,
        });
    }
    Ok(result)
}

struct PressedKeys {
    page: chromiumoxide::Page,
    held: Vec<Key>,
    modifiers: i64,
    lock: Option<OwnedMutexGuard<()>>,
}

impl PressedKeys {
    async fn release(&mut self) -> anyhow::Result<()> {
        let mut modifiers = self.modifiers;
        let commands = self
            .held
            .iter()
            .rev()
            .map(|key| {
                modifiers &= !key.modifier;
                event(key, DispatchKeyEventType::KeyUp, modifiers)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        // Queue all releases before waiting; a busy renderer must not prevent later modifier releases.
        for result in futures::future::join_all(
            commands
                .into_iter()
                .map(|command| self.page.execute(command)),
        )
        .await
        {
            result?;
        }
        self.held.clear();
        self.modifiers = 0;
        Ok(())
    }
}

impl Drop for PressedKeys {
    fn drop(&mut self) {
        if self.held.is_empty() {
            return;
        }
        let mut cleanup = Self {
            page: self.page.clone(),
            held: std::mem::take(&mut self.held),
            modifiers: self.modifiers,
            lock: self.lock.take(),
        };
        runtime::runtime().spawn(async move {
            let result = tokio::time::timeout(Duration::from_secs(2), cleanup.release()).await;
            diagnostics::event(
                "keyboard_cleanup",
                if matches!(result, Ok(Ok(()))) {
                    "completed"
                } else {
                    "failed"
                },
            );
            // Do not recursively schedule another cleanup if the transport is gone.
            cleanup.held.clear();
        });
    }
}

fn event(
    key: &Key,
    kind: DispatchKeyEventType,
    modifiers: i64,
) -> anyhow::Result<DispatchKeyEventParams> {
    let definition = key.definition;
    let mut builder = DispatchKeyEventParams::builder()
        .r#type(kind.clone())
        .key(definition.key)
        .code(definition.code)
        .windows_virtual_key_code(definition.key_code)
        .modifiers(modifiers);
    if kind != DispatchKeyEventType::KeyUp && modifiers & 7 == 0 {
        if definition.key.chars().count() == 1 {
            builder = builder.text(definition.key);
        } else if let Some(text) = definition.text {
            builder = builder.text(text);
        }
    }
    builder.build().map_err(anyhow::Error::msg)
}

pub(super) async fn send_chord(
    page: chromiumoxide::Page,
    keys: Vec<Key>,
    lock: OwnedMutexGuard<()>,
) -> anyhow::Result<()> {
    let mut pressed = PressedKeys {
        page,
        held: Vec::new(),
        modifiers: 0,
        lock: Some(lock),
    };
    for key in keys {
        pressed.modifiers |= key.modifier;
        let command = event(&key, DispatchKeyEventType::KeyDown, pressed.modifiers)?;
        pressed.held.push(key);
        pressed.page.execute(command).await?;
    }
    pressed.release().await
}

/// Send input to a page; modifiers are separate arguments, as in computer-use and terminal-use.
#[pyclass(frozen, module = "browser_use")]
pub(crate) struct Keyboard {
    pub(super) page: Page,
}

#[pymethods]
impl Keyboard {
    /// Press and release a complete chord, e.g. press("ctrl", "shift", "a").
    #[pyo3(signature = (*keys, timeout=10.0))]
    fn press(&self, py: Python<'_>, keys: Vec<String>, timeout: f64) -> PyResult<()> {
        let keys = chord(keys)?;
        let lock = self.page.resources.keyboard.clone();
        self.page
            .run(py, "keyboard_press", timeout, |page| async move {
                send_chord(page, keys, lock.lock_owned().await).await
            })
    }

    /// Insert literal Unicode text at the current focus, without interpreting it as key names.
    #[pyo3(signature = (text, *, timeout=10.0))]
    fn insert_text(&self, py: Python<'_>, text: String, timeout: f64) -> PyResult<()> {
        let lock: Arc<Mutex<()>> = self.page.resources.keyboard.clone();
        self.page
            .run(py, "keyboard_insert_text", timeout, |page| async move {
                let _guard = lock.lock_owned().await;
                page.execute(InsertTextParams::new(text)).await?;
                Ok(())
            })
    }
}
