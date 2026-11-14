use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const FILES: [&str; 2] = ["design.json", ".design/workspace.json"];

#[derive(Clone, Debug, PartialEq)]
pub struct Documents {
    pub design: Value,
    pub workspace: Value,
}

pub struct LoadedDocuments {
    pub documents: Documents,
    pub bytes: [Vec<u8>; 2],
}

#[derive(Deserialize)]
#[expect(
    dead_code,
    reason = "Deserialization validates the on-disk contract without rebuilding its JSON"
)]
struct Design {
    name: String,
    elements: BTreeMap<String, Element>,
    parameters: BTreeMap<String, Parameter>,
}

#[derive(Deserialize)]
#[expect(dead_code, reason = "Deserialization validates element fields")]
struct Element {
    text: Option<String>,
    style: Option<BTreeMap<String, StyleValue>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
#[expect(dead_code, reason = "Deserialization validates style value types")]
enum StyleValue {
    String(String),
    Number(serde_json::Number),
}

#[derive(Deserialize)]
#[expect(dead_code, reason = "Deserialization validates parameter fields")]
struct Parameter {
    label: Option<String>,
    control: Control,
    value: ParameterValue,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    options: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Control {
    Range,
    Number,
    Color,
    Text,
    Boolean,
    Select,
}

#[derive(Deserialize)]
#[serde(untagged)]
#[expect(dead_code, reason = "Deserialization validates parameter value types")]
enum ParameterValue {
    String(String),
    Number(serde_json::Number),
    Boolean(bool),
}

#[derive(Serialize, Deserialize)]
pub struct Artboard {
    pub id: String,
    name: String,
    route: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    text: String,
    resolved: bool,
    anchor: Anchor,
}

#[derive(Serialize, Deserialize)]
struct Anchor {
    artboard: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    element: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y: Option<f64>,
}

#[derive(Deserialize)]
#[expect(dead_code, reason = "Deserialization validates workspace fields")]
struct Workspace {
    artboards: Vec<Artboard>,
    comments: Vec<Comment>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Edit {
    Text {
        element: String,
        text: String,
    },
    Style {
        element: String,
        style: BTreeMap<String, Value>,
    },
    Parameter {
        name: String,
        value: Value,
    },
    Artboard {
        artboard: Artboard,
    },
    RemoveArtboard {
        id: String,
    },
    Comment {
        comment: Comment,
    },
    RemoveComment {
        id: String,
    },
}

impl Documents {
    pub fn value(&self, index: usize) -> &Value {
        if index == 0 {
            &self.design
        } else {
            &self.workspace
        }
    }

    pub fn validate(&self) -> Result<()> {
        // Deserialize for the contract while retaining the original JSON, including extra fields.
        serde_json::from_value::<Design>(self.design.clone()).context("design.json")?;
        serde_json::from_value::<Workspace>(self.workspace.clone())
            .context(".design/workspace.json")?;
        self.design
            .as_object()
            .context("design.json must be an object")?;
        for field in ["elements", "parameters"] {
            let entries = self.design[field]
                .as_object()
                .with_context(|| format!("design.{field} must be an object"))?;
            for (name, value) in entries {
                value
                    .as_object()
                    .with_context(|| format!("design.{field}.{name} must be an object"))?;
            }
        }
        self.workspace
            .as_object()
            .context("workspace must be an object")?;
        Ok(())
    }

    pub fn edit(&mut self, edits: Vec<Edit>) -> Result<()> {
        for edit in edits {
            match edit {
                Edit::Text { element, text } => {
                    let elements = self.design["elements"].as_object_mut().unwrap();
                    let target = elements.entry(element).or_insert_with(|| json!({}));
                    target["text"] = json!(text);
                }
                Edit::Style { element, style } => {
                    let elements = self.design["elements"].as_object_mut().unwrap();
                    let target = elements.entry(element).or_insert_with(|| json!({}));
                    let styles = target
                        .as_object_mut()
                        .unwrap()
                        .entry("style")
                        .or_insert_with(|| json!({}))
                        .as_object_mut()
                        .context("Element style must be an object")?;
                    for (key, value) in style {
                        if value.is_null() {
                            styles.remove(&key);
                        } else if value.is_string() || value.is_number() {
                            styles.insert(key, value);
                        } else {
                            bail!("Style values must be strings, numbers, or null");
                        }
                    }
                }
                Edit::Parameter { name, value } => {
                    if !(value.is_string() || value.is_number() || value.is_boolean()) {
                        bail!("Parameter values must be strings, numbers, or booleans");
                    }
                    let parameter = self.design["parameters"]
                        .get_mut(&name)
                        .with_context(|| format!("Unknown parameter: {name}"))?;
                    parameter["value"] = value;
                }
                Edit::Artboard { artboard } => {
                    upsert(
                        &mut self.workspace["artboards"],
                        &artboard.id,
                        serde_json::to_value(&artboard)?,
                    );
                }
                Edit::RemoveArtboard { id } => remove(&mut self.workspace["artboards"], &id),
                Edit::Comment { comment } => {
                    upsert(
                        &mut self.workspace["comments"],
                        &comment.id,
                        serde_json::to_value(&comment)?,
                    );
                }
                Edit::RemoveComment { id } => remove(&mut self.workspace["comments"], &id),
            }
        }
        self.validate()
    }
}

fn upsert(array: &mut Value, id: &str, value: Value) {
    let array = array.as_array_mut().unwrap();
    if let Some(existing) = array.iter_mut().find(|item| item["id"] == id) {
        *existing = value;
    } else {
        array.push(value);
    }
}

fn remove(array: &mut Value, id: &str) {
    array
        .as_array_mut()
        .unwrap()
        .retain(|item| item["id"] != id);
}

pub fn read(root: &Path) -> Result<LoadedDocuments> {
    let bytes = [
        fs::read(root.join(FILES[0]))
            .with_context(|| format!("Read {}", root.join(FILES[0]).display()))?,
        fs::read(root.join(FILES[1]))
            .with_context(|| format!("Read {}", root.join(FILES[1]).display()))?,
    ];
    let documents = Documents {
        design: serde_json::from_slice(&bytes[0]).context("Invalid JSON in design.json")?,
        workspace: serde_json::from_slice(&bytes[1])
            .context("Invalid JSON in .design/workspace.json")?,
    };
    documents.validate()?;
    Ok(LoadedDocuments { documents, bytes })
}

pub fn create_missing(root: &Path, name: &str) -> Result<()> {
    fs::create_dir_all(root.join(".design"))?;
    let defaults = [
        json!({ "name": name, "elements": {}, "parameters": {} }),
        json!({
            "artboards": [{
                "id": "default", "name": "Default", "route": "/",
                "x": 80, "y": 80, "width": 1200, "height": 800
            }],
            "comments": []
        }),
    ];
    for (file, default) in FILES.iter().zip(defaults) {
        let path = root.join(file);
        if !path.try_exists()? {
            let mut temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
            write_json(&mut temporary, &default)?;
            match temporary.persist_noclobber(&path) {
                Ok(_) => (),
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(())
}

pub fn write_json(writer: &mut impl Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    write_json(&mut temporary, value)?;
    temporary.persist(path)?;
    Ok(())
}

pub fn write_changed(root: &Path, loaded: &LoadedDocuments, next: &Documents) -> Result<()> {
    let mut pending: Vec<(usize, PathBuf, tempfile::NamedTempFile)> = Vec::new();
    for (index, file) in FILES.iter().enumerate() {
        if loaded.documents.value(index) == next.value(index) {
            continue;
        }
        let path = root.join(file);
        let mut temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        temporary
            .as_file()
            .set_permissions(fs::metadata(&path)?.permissions())?;
        write_json(&mut temporary, next.value(index))?;
        pending.push((index, path, temporary));
    }
    // Stage every output first. Do not replace files changed since the actor's reload.
    for (index, file) in FILES.iter().enumerate() {
        if fs::read(root.join(file))? != loaded.bytes[index] {
            bail!(
                "{file} changed while editing; no staged writes were applied. Read state and retry"
            );
        }
    }
    for (index, path, temporary) in pending {
        if fs::read(&path)? != loaded.bytes[index] {
            bail!(
                "{} changed while editing; read state and retry",
                path.display()
            );
        }
        temporary.persist(&path)?;
    }
    Ok(())
}
