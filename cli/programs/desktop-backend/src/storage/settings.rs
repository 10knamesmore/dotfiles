use std::{collections::BTreeMap, env, io::ErrorKind, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoMount {
    #[default]
    Inherit,
    On,
    Off,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preference {
    pub auto_mount: AutoMount,
    pub read_only: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub auto_mount: bool,
    pub notifications: bool,
    pub volumes: BTreeMap<String, Preference>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_mount: false,
            notifications: true,
            volumes: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn preference(&self, key: &str) -> Preference {
        self.volumes.get(key).cloned().unwrap_or_default()
    }

    pub fn should_mount(&self, key: &str) -> bool {
        match self.preference(key).auto_mount {
            AutoMount::Inherit => self.auto_mount,
            AutoMount::On => true,
            AutoMount::Off => false,
        }
    }
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub async fn load() -> Result<(Self, Settings)> {
        let state = match env::var_os("XDG_STATE_HOME").filter(|value| !value.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => {
                PathBuf::from(env::var_os("HOME").context("HOME is not set")?).join(".local/state")
            }
        };
        let path = state.join("desktop-backend/storage.json");
        let settings = match fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .with_context(|| format!("read {}", path.display()))?,
            Err(error) if error.kind() == ErrorKind::NotFound => Settings::default(),
            Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
        };
        Ok((Self { path }, settings))
    }

    pub async fn save(&self, settings: &Settings) -> Result<()> {
        let directory = self.path.parent().unwrap();
        fs::create_dir_all(directory).await?;
        let temporary = tempfile::NamedTempFile::new_in(directory)?;
        let mut file = fs::File::from_std(temporary.reopen()?);
        file.write_all(&serde_json::to_vec_pretty(settings)?)
            .await?;
        file.sync_all().await?;
        temporary.persist(&self.path)?;
        Ok(())
    }
}
