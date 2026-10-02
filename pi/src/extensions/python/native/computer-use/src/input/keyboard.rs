//! Resolve named chords with XKB and generate Unicode-only text keymaps.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::os::fd::AsFd;

use pyo3::prelude::*;
use xkbcommon::xkb;

use crate::wait;
use crate::wayland::VirtualKeyboard;

thread_local! {
    static KEYMAP: RefCell<Option<xkb::Keymap>> = const { RefCell::new(None) };
}

/// A validated US base key, independent of any connected input device.
#[derive(Clone)]
pub(super) struct Key {
    /// XKB keycode, eight greater than the evdev code sent over Wayland.
    pub code: u32,

    /// Canonical keysym name returned by held_keys().
    pub name: String,

    /// Whether XKB treats this key as a modifier, so chords press it first.
    modifier: bool,
}

pub(super) fn keymap() -> PyResult<xkb::Keymap> {
    KEYMAP.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
            *slot = Some(
                xkb::Keymap::new_from_names(
                    &context,
                    "evdev",
                    "pc105",
                    "us",
                    "",
                    Some(String::new()),
                    xkb::KEYMAP_COMPILE_NO_FLAGS,
                )
                .ok_or_else(|| {
                    wait::runtime("libxkbcommon could not load the evdev/pc105/us keymap")
                })?,
            );
        }
        Ok(slot.as_ref().unwrap().clone())
    })
}

pub(super) fn resolve(py: Python<'_>, names: &[String]) -> PyResult<Vec<Key>> {
    if names.is_empty() {
        return Err(wait::invalid("at least one key is required"));
    }
    let map = keymap()?;
    let mut keys: Vec<Key> = Vec::new();
    for name in names {
        py.check_signals()?;
        let normalized = name.to_ascii_lowercase();
        let alias = match normalized.as_str() {
            "ctrl" => "Control_L",
            "shift" => "Shift_L",
            "alt" => "Alt_L",
            "super" => "Super_L",
            "enter" => "Return",
            "escape" | "esc" => "Escape",
            "tab" => "Tab",
            "backspace" => "BackSpace",
            "delete" => "Delete",
            "insert" => "Insert",
            "home" => "Home",
            "end" => "End",
            "left" => "Left",
            "right" => "Right",
            "up" => "Up",
            "down" => "Down",
            "pageup" => "Prior",
            "pagedown" => "Next",
            "space" => "space",
            _ => name.as_str(),
        };
        if alias.contains('\0') {
            return Err(wait::invalid("key names cannot contain NUL"));
        }
        let lower;
        let alias = if alias.len() == 1 && alias.as_bytes()[0].is_ascii_uppercase() {
            lower = alias.to_ascii_lowercase();
            lower.as_str()
        } else {
            alias
        };
        let symbol = xkb::keysym_from_name(alias, xkb::KEYSYM_CASE_INSENSITIVE);
        let code = if symbol.raw() != 0 {
            (map.min_keycode().raw()..=map.max_keycode().raw()).find(|code| {
                map.key_get_syms_by_level((*code).into(), 0, 0)
                    .contains(&symbol)
            })
        } else {
            None
        };
        let code = code.ok_or_else(|| {
            wait::invalid(format!(
                "unknown base key {name:?}; use XKB names, explicit modifiers, or type_text()"
            ))
        })?;
        if !keys.iter().any(|key| key.code == code) {
            let mut state = xkb::State::new(&map);
            state.update_key(code.into(), xkb::KeyDirection::Down);
            let modifier = state.serialize_mods(
                xkb::STATE_MODS_DEPRESSED | xkb::STATE_MODS_LATCHED | xkb::STATE_MODS_LOCKED,
            ) != 0;
            keys.push(Key {
                code,
                name: xkb::keysym_get_name(symbol),
                modifier,
            });
        }
    }
    keys.sort_by_key(|key| !key.modifier);
    Ok(keys)
}

pub(super) fn upload(device: &VirtualKeyboard, map: &xkb::Keymap) -> PyResult<()> {
    let mut bytes = map.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1).into_bytes();
    bytes.push(0);
    let mut file = tempfile::tempfile().map_err(wait::runtime)?;
    file.write_all(&bytes).map_err(wait::runtime)?;
    device.keymap(1, file.as_fd(), bytes.len() as u32);
    Ok(())
}

/// Keep the named-key device and its worker-local XKB modifier state together.
pub(super) struct Keyboard {
    /// Virtual keyboard with the US keymap already installed.
    pub device: VirtualKeyboard,

    /// Includes only this device's depressed, latched and locked modifiers.
    pub state: xkb::State,
}

impl Keyboard {
    pub(super) fn key(&mut self, code: u32, down: bool, time: u32) {
        self.device.key(time, code - 8, u32::from(down));
        self.state.update_key(
            code.into(),
            if down {
                xkb::KeyDirection::Down
            } else {
                xkb::KeyDirection::Up
            },
        );
        self.device.modifiers(
            self.state.serialize_mods(xkb::STATE_MODS_DEPRESSED),
            self.state.serialize_mods(xkb::STATE_MODS_LATCHED),
            self.state.serialize_mods(xkb::STATE_MODS_LOCKED),
            self.state.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE),
        );
    }

    pub(super) fn reset_modifiers(&mut self) -> PyResult<()> {
        self.state = xkb::State::new(&keymap()?);
        self.device.modifiers(0, 0, 0, 0);
        Ok(())
    }
}

/// One text keymap with at most 200 distinct characters, avoiding extended keycodes.
pub(super) struct TextChunk {
    /// A validated one-level Unicode map, installed before this chunk is typed.
    pub map: xkb::Keymap,

    /// Evdev keycodes in text order; starts at one because zero is KEY_RESERVED.
    pub codes: Vec<u32>,
}

/// Build every map before emitting keys, so invalid text cannot partially type.
pub(super) fn text_chunks(py: Python<'_>, text: &str) -> PyResult<Vec<TextChunk>> {
    let mut chunks = Vec::new();
    let mut symbols = HashMap::new();
    let mut order = Vec::new();
    let mut codes = Vec::new();
    for (index, character) in text.chars().enumerate() {
        if index % 200 == 0 {
            py.check_signals()?;
        }
        if character.is_control() && !matches!(character, '\n' | '\t') {
            return Err(wait::invalid(
                "type_text accepts printable Unicode, newline and tab; use press for other control keys",
            ));
        }
        if symbols.len() == 200 && !symbols.contains_key(&character) {
            chunks.push(text_chunk(&order, std::mem::take(&mut codes))?);
            symbols.clear();
            order.clear();
        }
        let next_code = symbols.len() as u32 + 9;
        let code = *symbols.entry(character).or_insert_with(|| {
            order.push(character);
            next_code
        });
        codes.push(code - 8);
    }
    if !codes.is_empty() {
        chunks.push(text_chunk(&order, codes)?);
    }
    Ok(chunks)
}

fn text_chunk(characters: &[char], codes: Vec<u32>) -> PyResult<TextChunk> {
    use std::fmt::Write;
    let mut source =
        String::from("xkb_keymap { xkb_keycodes \"text\" { minimum = 8; maximum = 255;");
    for index in 0..characters.len() {
        let _ = write!(source, "<T{index:03}> = {};", index + 9);
    }
    source.push_str("}; xkb_types \"text\" { type \"ONE_LEVEL\" { modifiers = None; map[None] = Level1; }; }; xkb_compatibility \"text\" {}; xkb_symbols \"text\" {");
    for (index, character) in characters.iter().enumerate() {
        let symbol = match character {
            '\n' => "Return".into(),
            '\t' => "Tab".into(),
            character => format!("U{:04X}", *character as u32),
        };
        let _ = write!(
            source,
            "key <T{index:03}> {{ type[Group1] = \"ONE_LEVEL\", symbols[Group1] = [ {symbol} ] }};"
        );
    }
    source.push_str("}; };");
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let map = xkb::Keymap::new_from_string(
        &context,
        source,
        xkb::KEYMAP_FORMAT_TEXT_V1,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .ok_or_else(|| wait::runtime("libxkbcommon could not compile the Unicode keymap"))?;
    Ok(TextChunk { map, codes })
}
