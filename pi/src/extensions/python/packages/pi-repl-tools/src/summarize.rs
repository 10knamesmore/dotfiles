//! Summarize retained Python bindings without invoking custom object representations.

use std::time::Instant;

use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyBytes, PyComplex, PyDict, PyFloat, PyFrozenSet, PyFunction, PyInt, PyList, PyModule,
    PySet, PyString, PyTuple,
};

use crate::diagnostics;

const MAX_SUMMARY: usize = 240;
const PREVIEW_ITEMS: usize = 4;

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(_make_summarize, module)?)?;
    Ok(())
}

/// Bind the native summary function to one namespace, hiding unchanged worker bindings.
#[pyfunction]
#[pyo3(signature = (namespace, /))]
fn _make_summarize<'py>(namespace: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    let py = namespace.py();
    let injected = namespace.copy()?;
    injected.set_item("__builtins__", py.import("builtins")?.dict())?;
    // Python's partial and dict own the reference cycle, so Python GC can collect it.
    let callback = py.import("functools")?.getattr("partial")?.call1((
        wrap_pyfunction!(summarize, py)?,
        namespace,
        &injected,
    ))?;
    injected.set_item("summarize", &callback)?;
    diagnostics::event("summarize_bound");
    Ok(callback)
}

/// Print retained names, Python types and bounded value summaries to the cell's stdout.
#[pyfunction]
#[pyo3(signature = (namespace, injected, /))]
fn summarize(namespace: &Bound<'_, PyDict>, injected: &Bound<'_, PyDict>) -> PyResult<()> {
    let started = Instant::now();
    diagnostics::event("summarize_started");
    let result = print_summary(namespace, injected);
    match &result {
        Ok(count) => diagnostics::event(&format!(
            "summarize_completed bindings={count} elapsed_ms={}",
            started.elapsed().as_millis()
        )),
        Err(_) => diagnostics::event(&format!(
            "summarize_failed elapsed_ms={}",
            started.elapsed().as_millis()
        )),
    }
    result.map(|_| ())
}

/// Hold one binding's display fields without retaining its Python value.
struct SummaryRow {
    /// Name available to subsequent cells.
    name: String,

    /// Python type, qualified by its defining module for non-built-in classes.
    type_name: String,

    /// Bounded value or shape metadata.
    summary: String,
}

fn print_summary(namespace: &Bound<'_, PyDict>, injected: &Bound<'_, PyDict>) -> PyResult<usize> {
    let py = namespace.py();
    let formatter = SummaryFormatter::new(py)?;
    let mut rows = Vec::new();
    for (name, value) in namespace.copy()?.iter() {
        py.check_signals()?;
        if injected
            .get_item(&name)?
            .is_some_and(|original| original.is(&value))
        {
            continue;
        }
        rows.push(SummaryRow {
            name: name.extract()?,
            type_name: type_name(&value)?,
            summary: truncate(
                formatter
                    .summary(&value)?
                    .replace('\n', "\\n")
                    .replace('\r', "\\r"),
                MAX_SUMMARY,
            ),
        });
    }
    rows.sort_by(|left, right| left.name.cmp(&right.name));
    let count = rows.len();
    rows.insert(
        0,
        SummaryRow {
            name: "name".into(),
            type_name: "type".into(),
            summary: "summary".into(),
        },
    );
    let name_width = rows
        .iter()
        .map(|row| row.name.chars().count())
        .max()
        .unwrap();
    let type_width = rows
        .iter()
        .map(|row| row.type_name.chars().count())
        .max()
        .unwrap();
    let output = rows
        .iter()
        .map(|row| {
            format!(
                "{:<name_width$}  {:<type_width$}  {}",
                row.name, row.type_name, row.summary
            )
            .trim_end()
            .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    py.import("builtins")?.getattr("print")?.call1((output,))?;
    Ok(count)
}

fn type_name(value: &Bound<'_, PyAny>) -> PyResult<String> {
    value.get_type().fully_qualified_name()?.extract()
}

fn truncate(mut text: String, limit: usize) -> String {
    if text.chars().count() > limit {
        let end = text.char_indices().nth(limit - 3).unwrap().0;
        text.truncate(end);
        text.push_str("...");
    }
    text
}

/// Reuse Python's scalar formatting and loaded module types for one summary call.
struct SummaryFormatter<'py> {
    /// Standard-library formatter for bounded str and int representations.
    repr: Bound<'py, PyAny>,

    /// Already loaded modules; summaries do not import optional data libraries.
    modules: Bound<'py, PyDict>,
}

impl<'py> SummaryFormatter<'py> {
    fn new(py: Python<'py>) -> PyResult<Self> {
        let repr = py.import("reprlib")?.getattr("Repr")?.call0()?;
        repr.setattr("maxstring", 80)?;
        repr.setattr("maxlong", 80)?;
        Ok(Self {
            repr,
            modules: py.import("sys")?.getattr("modules")?.cast_into()?,
        })
    }

    fn scalar(value: &Bound<'_, PyAny>) -> bool {
        value.is_exact_instance_of::<PyString>()
            || value.is_exact_instance_of::<PyBytes>()
            || value.is_exact_instance_of::<PyInt>()
            || value.is_exact_instance_of::<PyFloat>()
            || value.is_exact_instance_of::<PyComplex>()
            || value.is_exact_instance_of::<PyBool>()
            || value.is_none()
    }

    fn sequence(value: &Bound<'_, PyAny>) -> bool {
        value.is_exact_instance_of::<PyList>() || value.is_exact_instance_of::<PyTuple>()
    }

    fn loaded_type(&self, value: &Bound<'_, PyAny>, module: &str, name: &str) -> PyResult<bool> {
        match self.modules.get_item(module)? {
            Some(module) if !module.is_none() => {
                Ok(value.is_exact_instance(&module.getattr(name)?))
            }
            _ => Ok(false),
        }
    }

    fn summary(&self, value: &Bound<'_, PyAny>) -> PyResult<String> {
        if Self::scalar(value) {
            return self.preview(value, 2);
        }
        if Self::sequence(value) {
            let length = value.len()?;
            return Ok(if length == 0 {
                "len=0".into()
            } else {
                format!(
                    "len={length}, first={}",
                    self.preview(&value.get_item(0)?, 2)?
                )
            });
        }
        if value.is_exact_instance_of::<PyDict>() {
            return Ok(format!(
                "len={}, preview={}",
                value.len()?,
                self.preview(value, 2)?
            ));
        }
        if value.is_exact_instance_of::<PySet>()
            || value.is_exact_instance_of::<PyFrozenSet>()
            || value.is_exact_instance(&value.py().import("builtins")?.getattr("range")?)
        {
            return Ok(format!("len={}", value.len()?));
        }
        if value.is_exact_instance_of::<PyModule>() {
            return value.getattr("__name__")?.extract();
        }
        if value.is_exact_instance_of::<PyFunction>() {
            return self.function_signature(value);
        }
        if self.loaded_type(value, "numpy", "ndarray")? {
            return Ok(format!(
                "shape={}, dtype={}",
                value.getattr("shape")?,
                value.getattr("dtype")?
            ));
        }
        if self.loaded_type(value, "pandas", "DataFrame")? {
            let columns = value
                .getattr("columns")?
                .try_iter()?
                .take(PREVIEW_ITEMS + 1)
                .collect::<PyResult<Vec<_>>>()?;
            let columns = PyList::new(value.py(), columns)?;
            return Ok(format!(
                "shape={}, columns={}",
                value.getattr("shape")?,
                self.preview(columns.as_any(), 2)?
            ));
        }
        Ok(String::new())
    }

    /// Traverse only exact built-in containers, limiting both depth and item count.
    fn preview(&self, value: &Bound<'_, PyAny>, depth: usize) -> PyResult<String> {
        if value.is_exact_instance_of::<PyString>() || value.is_exact_instance_of::<PyInt>() {
            return self.repr.call_method1("repr", (value,))?.extract();
        }
        if let Ok(bytes) = value.cast_exact::<PyBytes>() {
            let data = bytes.as_bytes();
            let prefix = PyBytes::new(value.py(), &data[..data.len().min(60)]).repr()?;
            return Ok(format!(
                "{prefix}{}",
                if data.len() > 60 { "..." } else { "" }
            ));
        }
        if Self::scalar(value) {
            return value.repr()?.extract();
        }
        let is_dict = value.is_exact_instance_of::<PyDict>();
        if !is_dict && !Self::sequence(value) {
            return Ok(format!("<{}>", type_name(value)?));
        }
        let length = value.len()?;
        if depth == 0 {
            return Ok(format!("<{} len={length}>", type_name(value)?));
        }
        let mut parts = Vec::new();
        let (left, right) = if is_dict {
            for (key, item) in value.cast::<PyDict>()?.iter().take(PREVIEW_ITEMS) {
                parts.push(format!(
                    "{}: {}",
                    self.preview(&key, depth - 1)?,
                    self.preview(&item, depth - 1)?
                ));
            }
            ("{", "}")
        } else {
            for index in 0..length.min(PREVIEW_ITEMS) {
                parts.push(self.preview(&value.get_item(index)?, depth - 1)?);
            }
            if value.is_exact_instance_of::<PyList>() {
                ("[", "]")
            } else if length == 1 {
                ("(", ",)")
            } else {
                ("(", ")")
            }
        };
        if length > PREVIEW_ITEMS {
            parts.push("...".into());
        }
        Ok(format!("{left}{}{right}", parts.join(", ")))
    }

    /// Keep Python signature discovery, but format defaults without custom repr calls.
    fn function_signature(&self, function: &Bound<'_, PyAny>) -> PyResult<String> {
        let py = function.py();
        let inspect = py.import("inspect")?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("follow_wrapped", false)?;
        kwargs.set_item("eval_str", false)?;
        let signature = inspect
            .getattr("signature")?
            .call((function,), Some(&kwargs))?;
        let parameters = signature
            .getattr("parameters")?
            .call_method0("values")?
            .try_iter()?
            .collect::<PyResult<Vec<_>>>()?;
        let parameter_type = inspect.getattr("Parameter")?;
        let empty = parameter_type.getattr("empty")?;
        let mut parts = Vec::new();
        let mut has_varargs = false;
        for (index, parameter) in parameters.iter().enumerate() {
            let kind = parameter.getattr("kind")?;
            let keyword_only = kind.is(&parameter_type.getattr("KEYWORD_ONLY")?);
            if keyword_only && !has_varargs {
                parts.push("*".into());
                has_varargs = true;
            }
            let prefix = if kind.is(&parameter_type.getattr("VAR_POSITIONAL")?) {
                has_varargs = true;
                "*"
            } else if kind.is(&parameter_type.getattr("VAR_KEYWORD")?) {
                "**"
            } else {
                ""
            };
            let name: String = parameter.getattr("name")?.extract()?;
            let mut text = format!("{prefix}{name}");
            let default = parameter.getattr("default")?;
            if !default.is(&empty) {
                text.push('=');
                text.push_str(&self.preview(&default, 2)?);
            }
            parts.push(text);
            if kind.is(&parameter_type.getattr("POSITIONAL_ONLY")?) {
                let last = match parameters.get(index + 1) {
                    Some(next) => !next.getattr("kind")?.is(&kind),
                    None => true,
                };
                if last {
                    parts.push("/".into());
                }
            }
        }
        Ok(format!("({})", parts.join(", ")))
    }
}
