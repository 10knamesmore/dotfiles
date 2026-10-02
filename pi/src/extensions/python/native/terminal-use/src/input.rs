//! Python-facing input parsing and terminal byte encoding.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyBytes, PyMapping, PyMemoryView, PySequence};
use pyo3::types::{PyByteArray, PyString};

use crate::model::{CursorKey, InputSpec, KeySpec, Rect, WaitSpec};

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
                ensure_fields(mapping, &["kind", "key"])?;
                let key = extract_string(&mapping.get_item("key")?, &format!("event {index} key"))?;
                InputSpec::Key(parse_key(&key)?)
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

pub(crate) fn encode_key_input(key: &str) -> PyResult<InputSpec> {
    Ok(InputSpec::Key(parse_key(key)?))
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

fn parse_key(key: &str) -> PyResult<KeySpec> {
    if key.is_empty() {
        return Err(value_error("key must not be empty"));
    }
    let cursor = match key.to_ascii_lowercase().as_str() {
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
    let named = match key.to_ascii_lowercase().as_str() {
        "enter" | "return" => Some(b"\r".as_slice()),
        "tab" => Some(b"\t".as_slice()),
        "escape" | "esc" => Some(b"\x1b".as_slice()),
        "backspace" | "bspace" => Some(b"\x7f".as_slice()),
        "delete" | "dc" => Some(b"\x1b[3~".as_slice()),
        "insert" | "ic" => Some(b"\x1b[2~".as_slice()),
        "pageup" | "pgup" | "ppage" => Some(b"\x1b[5~".as_slice()),
        "pagedown" | "pgdn" | "npage" => Some(b"\x1b[6~".as_slice()),
        "space" => Some(b" ".as_slice()),
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
    if key.chars().count() == 1 {
        return Ok(KeySpec::Bytes(key.as_bytes().to_vec()));
    }
    if let Some((modifier, character)) = key.split_once('-') {
        match modifier.to_ascii_uppercase().as_str() {
            "C" => {
                let code = control_code(character)?;
                return Ok(KeySpec::Bytes(vec![code]));
            }
            "M" if character.chars().count() == 1 => {
                let mut bytes = vec![0x1b];
                bytes.extend(character.as_bytes());
                return Ok(KeySpec::Bytes(bytes));
            }
            _ => {}
        }
    }
    Err(value_error(format!(
        "unknown key {key:?}; use a single character, a named key, C-<character>, or M-<character>"
    )))
}

fn control_code(character: &str) -> PyResult<u8> {
    if character.eq_ignore_ascii_case("space") || character == " " {
        return Ok(0);
    }
    if character == "?" {
        return Ok(0x7f);
    }
    if character.chars().count() == 1 {
        let upper = character.as_bytes()[0].to_ascii_uppercase();
        if (b'@'..=b'_').contains(&upper) {
            return Ok(upper & 0x1f);
        }
    }
    Err(value_error(format!(
        "unsupported control key C-{character}; use a letter, Space, ?, or one of @ [ \\ ] ^ _"
    )))
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
