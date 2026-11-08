use crate::documents;
use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Registry {
    pub projects: Vec<Project>,
}

pub fn list(agent_dir: &Path) -> Result<Registry> {
    let path = agent_dir.join("design/projects.json");
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("Invalid design/projects.json"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Registry::default()),
        Err(error) => Err(error.into()),
    }
}

pub fn register(agent_dir: &Path, path: &Path, name: Option<&str>) -> Result<Project> {
    let directory = agent_dir.join("design");
    fs::create_dir_all(&directory)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("projects.lock"))?;
    lock.lock_exclusive()?;
    let mut registry = list(agent_dir)?;
    let path = path.to_str().context("Project path must be valid UTF-8")?;
    let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, path.as_bytes()).to_string();
    let project = if let Some(project) = registry
        .projects
        .iter_mut()
        .find(|project| project.id == id)
    {
        if let Some(name) = name {
            project.name = name.to_owned();
        }
        project.clone()
    } else {
        let project = Project {
            id,
            name: name.map(str::to_owned).unwrap_or_else(|| {
                Path::new(path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Design".to_owned())
            }),
            path: path.to_owned(),
        };
        registry.projects.push(project.clone());
        project
    };
    documents::atomic_json(&directory.join("projects.json"), &registry)?;
    Ok(project)
}
