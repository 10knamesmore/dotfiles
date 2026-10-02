//! Python-facing input parsing and terminal byte encoding.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyBytes, PyMapping, PyMemoryView, PySequence};
use pyo3::types::{PyByteArray, PyString};

use crate::model::{CursorKey, InputSpec, KeySpec, PixelSize, Rect, WaitSpec};

/// A zero-based terminal-cell rectangle accepted by the Python API.
#[pyclass(name = "Rect", frozen, module = "terminal_use", from_py_object)]
#[derive(Clone, Copy)]
pub(crate) struct PyRect {
    #[pyo3(get)]
    pub(crate) x: u16,
    #[pyo3(get)]
    pub(crate) y: u16,
    #[pyo3(get)]
    pub(crate) width: u16,
    #[pyo3(get)]
    pub(crate) height: u16,
}

#[pymethods]
impl PyRect {
    #[new]
    #[pyo3(signature = (x, y, width, height))]
    fn new(x: u16, y: u16, width: u16, height: u16) -> PyResult<Self> {
        Rect::from_tuple((x, y, width, height))
            .map(|rect| Self {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            })
            .map_err(value_error)
    }
}

pub(crate) fn parse_rect(value: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Rect>> {
    let Some(value) = value else { return Ok(None) };

    if let Ok(rect) = value.extract::<PyRef<'_, PyRect>>() {
        return Ok(Some(Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }));
    }

    if let Ok(sequence) = value.cast::<PySequence>()
        && sequence.len()? == 4
    {
        let values = (
            extract_u16(&sequence.get_item(0)?, "rect x")?,
            extract_u16(&sequence.get_item(1)?, "rect y")?,
            extract_u16(&sequence.get_item(2)?, "rect width")?,
            extract_u16(&sequence.get_item(3)?, "rect height")?,
        );
        return Rect::from_tuple(values).map(Some).map_err(value_error);
    }

    let mapping = value
        .cast::<PyMapping>()
        .map_err(|_| type_error("rect must be a Rect, a four-element sequence, or a mapping"))?;
    let values = (
        extract_u16(&required_field(mapping, "x")?, "rect x")?,
        extract_u16(&required_field(mapping, "y")?, "rect y")?,
        extract_u16(&required_field(mapping, "width")?, "rect width")?,
        extract_u16(&required_field(mapping, "height")?, "rect height")?,
    );
    Rect::from_tuple(values).map(Some).map_err(value_error)
}

/// Parse the optional `(width, height)` pixel size assigned to one terminal cell.
///
/// A missing value keeps the headless default of 8x16 pixels per cell.
pub(crate) fn parse_cell_size(value: Option<&Bound<'_, PyAny>>) -> PyResult<PixelSize> {
    let Some(value) = value else {
        return Ok(PixelSize {
            width: 8,
            height: 16,
        });
    };
    if value.is_instance_of::<PyString>() || value.is_instance_of::<PyBytes>() {
        return Err(type_error(
            "cell_size must be a (width, height) pair of positive ints",
        ));
    }
    let sequence = value
        .cast::<PySequence>()
        .map_err(|_| type_error("cell_size must be a (width, height) pair of positive ints"))?;
    if sequence.len()? != 2 {
        return Err(value_error(
            "cell_size must have exactly two items: width and height",
        ));
    }
    let width = extract_u16(&sequence.get_item(0)?, "cell_size width")?;
    let height = extract_u16(&sequence.get_item(1)?, "cell_size height")?;
    if width == 0 || height == 0 {
        return Err(value_error("cell_size width and height must be positive"));
    }
    Ok(PixelSize { width, height })
}

pub(crate) fn parse_wait(value: Option<&Bound<'_, PyAny>>) -> PyResult<WaitSpec> {
    let Some(value) = value else {
        return WaitSpec::new(None, "screen", false).map_err(value_error);
    };
    let mapping = value
        .cast::<PyMapping>()
        .map_err(|_| type_error("wait_for must be a mapping"))?;
    ensure_allowed_fields(mapping, &["contains", "regex", "source"])?;

    let contains = optional_field(mapping, "contains")?;
    let regex = optional_field(mapping, "regex")?;
    if contains.is_some() == regex.is_some() {
        return Err(value_error(
            "wait_for must contain exactly one of 'contains' or 'regex'",
        ));
    }
    let (pattern, is_regex) = match (contains, regex) {
        (Some(value), None) => (extract_string(&value, "wait pattern")?, false),
        (None, Some(value)) => (extract_string(&value, "wait pattern")?, true),
        _ => unreachable!(),
    };
    if pattern.is_empty() {
        return Err(value_error("wait pattern must not be empty"));
    }
    let source = optional_field(mapping, "source")?
        .map(|value| extract_string(&value, "wait_for source"))
        .transpose()?
        .unwrap_or_else(|| "screen".to_owned());
    WaitSpec::new(Some(pattern), &source, is_regex).map_err(value_error)
}

pub(crate) fn encode_events(events: &Bound<'_, PyAny>) -> PyResult<Vec<InputSpec>> {
    if events.is_instance_of::<PyString>()
        || events.is_instance_of::<PyBytes>()
        || events.is_instance_of::<PyByteArray>()
        || events.is_instance_of::<PyMemoryView>()
    {
        return Err(type_error("events must be a sequence of event mappings"));
    }
    let sequence = events
        .cast::<PySequence>()
        .map_err(|_| type_error("events must be a sequence of event mappings"))?;
    let mut encoded_events = Vec::with_capacity(sequence.len()?);
    for index in 0..sequence.len()? {
        let event = sequence.get_item(index)?;
        let mapping = event
            .cast::<PyMapping>()
            .map_err(|_| type_error(format!("event {index} must be a mapping")))?;
        let kind = extract_string(&mapping.get_item("kind")?, &format!("event {index} kind"))?;
        let input = match kind.as_str() {
            "key" => {
                ensure_fields(mapping, &["kind", "keys"])?;
                let keys: Vec<String> = mapping.get_item("keys")?.extract().map_err(|_| {
                    type_error(format!(
                        "event {index} keys must be a sequence of key names"
                    ))
                })?;
                encode_key_input(&keys)?
            }
            "text" => {
                ensure_fields(mapping, &["kind", "text"])?;
                let text =
                    extract_string(&mapping.get_item("text")?, &format!("event {index} text"))?;
                InputSpec::Bytes(text.into_bytes())
            }
            "paste" => {
                ensure_fields(mapping, &["kind", "text"])?;
                let text =
                    extract_string(&mapping.get_item("text")?, &format!("event {index} text"))?;
                let mut payload = Vec::with_capacity(text.len() + 12);
                payload.extend_from_slice(b"\x1b[200~");
                payload.extend(text.as_bytes());
                payload.extend_from_slice(b"\x1b[201~");
                InputSpec::Bytes(payload)
            }
            "raw" => {
                ensure_fields(mapping, &["kind", "data"])?;
                InputSpec::Bytes(extract_bytes(
                    &mapping.get_item("data")?,
                    &format!("event {index} data"),
                )?)
            }
            other => {
                return Err(value_error(format!(
                    "event {index} has unknown kind {other:?}; expected 'key', 'text', 'paste', or 'raw'"
                )));
            }
        };
        encoded_events.push(input);
    }
    Ok(encoded_events)
}

pub(crate) fn encode_text(text: &str) -> InputSpec {
    InputSpec::Bytes(text.as_bytes().to_vec())
}

/// Encode one chord without retaining any pressed-key state between calls.
pub(crate) fn encode_key_input(keys: &[String]) -> PyResult<InputSpec> {
    Ok(InputSpec::Key(parse_chord(keys)?))
}

pub(crate) fn encode_paste(text: &str) -> InputSpec {
    let mut payload = Vec::with_capacity(text.len() + 12);
    payload.extend_from_slice(b"\x1b[200~");
    payload.extend(text.as_bytes());
    payload.extend_from_slice(b"\x1b[201~");
    InputSpec::Bytes(payload)
}

/// Convert one semantic input event to bytes using the session's current modes.
pub(crate) fn encode_input(input: &InputSpec, application_cursor: bool, output: &mut Vec<u8>) {
    match input {
        InputSpec::Bytes(bytes) => output.extend_from_slice(bytes),
        InputSpec::Key(KeySpec::Bytes(bytes)) => output.extend_from_slice(bytes),
        InputSpec::Key(KeySpec::Cursor(key)) => {
            output.extend_from_slice(match (key, application_cursor) {
                (CursorKey::Up, false) => b"\x1b[A",
                (CursorKey::Down, false) => b"\x1b[B",
                (CursorKey::Left, false) => b"\x1b[D",
                (CursorKey::Right, false) => b"\x1b[C",
                (CursorKey::Home, false) => b"\x1b[H",
                (CursorKey::End, false) => b"\x1b[F",
                (CursorKey::Up, true) => b"\x1bOA",
                (CursorKey::Down, true) => b"\x1bOB",
                (CursorKey::Left, true) => b"\x1bOD",
                (CursorKey::Right, true) => b"\x1bOC",
                (CursorKey::Home, true) => b"\x1bOH",
                (CursorKey::End, true) => b"\x1bOF",
            })
        }
    }
}

/// Modifiers belonging only to the current chord, not to the PTY session.
#[derive(Default)]
struct Modifiers {
    /// Encode the printable key as a control byte.
    ctrl: bool,

    /// Select the shifted character on a US keyboard before control encoding.
    shift: bool,

    /// Prefix the encoded character with Escape.
    alt: bool,
}

fn parse_chord(keys: &[String]) -> PyResult<KeySpec> {
    let mut modifiers = Modifiers::default();
    let mut base_key = None;
    for name in keys {
        let name = name.to_ascii_lowercase();
        match name.as_str() {
            "ctrl" => modifiers.ctrl = true,
            "shift" => modifiers.shift = true,
            "alt" => modifiers.alt = true,
            _ => {
                if base_key.replace(name).is_some() {
                    return Err(value_error(
                        "a terminal chord requires exactly one base key",
                    ));
                }
            }
        }
    }
    let key =
        base_key.ok_or_else(|| value_error("a terminal chord requires exactly one base key"))?;
    if let Some((plain, shifted)) = printable_key(&key) {
        let mut byte = if modifiers.shift { shifted } else { plain };
        if modifiers.ctrl {
            byte = control_code(byte)?;
        }
        let bytes = if modifiers.alt {
            vec![0x1b, byte]
        } else {
            vec![byte]
        };
        return Ok(KeySpec::Bytes(bytes));
    }
    let key = named_key(&key)?;
    if modifiers.ctrl || modifiers.shift || modifiers.alt {
        return Err(value_error(
            "modifiers on terminal function or navigation keys are not supported; use write() for explicit bytes",
        ));
    }
    Ok(key)
}

/// Return the unshifted and shifted characters for a US base key, never literal text.
fn printable_key(key: &str) -> Option<(u8, u8)> {
    if key.len() == 1 {
        let byte = key.as_bytes()[0];
        if byte.is_ascii_lowercase() {
            return Some((byte, byte.to_ascii_uppercase()));
        }
        if byte.is_ascii_digit() {
            return Some((byte, b")!@#$%^&*("[(byte - b'0') as usize]));
        }
    }
    Some(match key {
        "space" | " " => (b' ', b' '),
        "grave" | "`" => (b'`', b'~'),
        "minus" | "-" => (b'-', b'_'),
        "equal" | "=" => (b'=', b'+'),
        "bracketleft" | "[" => (b'[', b'{'),
        "bracketright" | "]" => (b']', b'}'),
        "backslash" | "\\" => (b'\\', b'|'),
        "semicolon" | ";" => (b';', b':'),
        "apostrophe" | "'" => (b'\'', b'"'),
        "comma" | "," => (b',', b'<'),
        "period" | "." => (b'.', b'>'),
        "slash" | "/" => (b'/', b'?'),
        _ => return None,
    })
}

fn named_key(key: &str) -> PyResult<KeySpec> {
    let cursor = match key {
        "up" => Some(CursorKey::Up),
        "down" => Some(CursorKey::Down),
        "left" => Some(CursorKey::Left),
        "right" => Some(CursorKey::Right),
        "home" => Some(CursorKey::Home),
        "end" => Some(CursorKey::End),
        _ => None,
    };
    if let Some(cursor) = cursor {
        return Ok(KeySpec::Cursor(cursor));
    }
    let named = match key {
        "enter" | "return" => Some(b"\r".as_slice()),
        "tab" => Some(b"\t".as_slice()),
        "escape" | "esc" => Some(b"\x1b".as_slice()),
        "backspace" => Some(b"\x7f".as_slice()),
        "delete" => Some(b"\x1b[3~".as_slice()),
        "insert" => Some(b"\x1b[2~".as_slice()),
        "pageup" | "prior" | "page_up" => Some(b"\x1b[5~".as_slice()),
        "pagedown" | "next" | "page_down" => Some(b"\x1b[6~".as_slice()),
        "f1" => Some(b"\x1bOP".as_slice()),
        "f2" => Some(b"\x1bOQ".as_slice()),
        "f3" => Some(b"\x1bOR".as_slice()),
        "f4" => Some(b"\x1bOS".as_slice()),
        "f5" => Some(b"\x1b[15~".as_slice()),
        "f6" => Some(b"\x1b[17~".as_slice()),
        "f7" => Some(b"\x1b[18~".as_slice()),
        "f8" => Some(b"\x1b[19~".as_slice()),
        "f9" => Some(b"\x1b[20~".as_slice()),
        "f10" => Some(b"\x1b[21~".as_slice()),
        "f11" => Some(b"\x1b[23~".as_slice()),
        "f12" => Some(b"\x1b[24~".as_slice()),
        _ => None,
    };
    if let Some(bytes) = named {
        return Ok(KeySpec::Bytes(bytes.to_vec()));
    }
    Err(value_error(format!(
        "unsupported terminal base key {key:?}; pass modifiers as separate names and use send_text() for literal text"
    )))
}

fn control_code(character: u8) -> PyResult<u8> {
    match character.to_ascii_uppercase() {
        b' ' => Ok(0),
        b'?' => Ok(0x7f),
        upper @ b'@'..=b'_' => Ok(upper & 0x1f),
        _ => Err(value_error(
            "this Ctrl combination has no supported terminal encoding; use write() for explicit bytes",
        )),
    }
}

fn required_field<'py>(mapping: &Bound<'py, PyMapping>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    if !mapping.contains(name)? {
        return Err(value_error(format!(
            "rect mapping is missing field {name:?}"
        )));
    }
    mapping.get_item(name)
}

fn ensure_fields(mapping: &Bound<'_, PyMapping>, expected: &[&str]) -> PyResult<()> {
    let keys: Vec<String> = mapping.keys()?.extract()?;
    let expected = expected
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let missing: Vec<_> = expected
        .iter()
        .filter(|field| !keys.iter().any(|key| key == **field))
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(value_error(format!(
            "mapping is missing fields {missing:?}"
        )));
    }
    ensure_allowed_keys(&keys, &expected)
}

fn ensure_allowed_fields(mapping: &Bound<'_, PyMapping>, allowed: &[&str]) -> PyResult<()> {
    let keys: Vec<String> = mapping.keys()?.extract()?;
    let allowed = allowed
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    ensure_allowed_keys(&keys, &allowed)
}

fn ensure_allowed_keys(keys: &[String], allowed: &std::collections::HashSet<&str>) -> PyResult<()> {
    let unexpected: Vec<_> = keys
        .iter()
        .filter(|key| !allowed.contains(key.as_str()))
        .collect();
    if !unexpected.is_empty() {
        return Err(value_error(format!(
            "mapping has unexpected fields {unexpected:?}"
        )));
    }
    Ok(())
}

fn optional_field<'py>(
    mapping: &Bound<'py, PyMapping>,
    name: &str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    if mapping.contains(name)? {
        Ok(Some(mapping.get_item(name)?))
    } else {
        Ok(None)
    }
}

fn extract_string(value: &Bound<'_, PyAny>, name: &str) -> PyResult<String> {
    value
        .extract()
        .map_err(|_| type_error(format!("{name} must be a string")))
}

fn extract_u16(value: &Bound<'_, PyAny>, name: &str) -> PyResult<u16> {
    if value.is_instance_of::<PyBool>() {
        return Err(type_error(format!("{name} must be an int")));
    }
    value
        .extract()
        .map_err(|_| type_error(format!("{name} must be an int")))
}

fn extract_bytes(value: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<u8>> {
    if let Ok(bytes) = value.cast::<PyBytes>() {
        return Ok(bytes.as_bytes().to_vec());
    }
    if let Ok(bytes) = value.cast::<PyByteArray>() {
        return Ok(unsafe { bytes.as_bytes() }.to_vec());
    }
    if let Ok(memoryview) = value.cast::<PyMemoryView>() {
        return memoryview
            .call_method0("tobytes")?
            .extract()
            .map_err(|_| type_error(format!("{name} must be bytes-like")));
    }
    Err(type_error(format!("{name} must be bytes-like")))
}

fn type_error(message: impl Into<String>) -> PyErr {
    PyTypeError::new_err(message.into())
}

fn value_error(message: impl Into<String>) -> PyErr {
    PyValueError::new_err(message.into())
}
