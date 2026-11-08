use crate::{
    documents::{self, Documents, Edit, LoadedDocuments},
    history::{Snapshot, describe_change},
    preview::Preview,
    registry::{self, Project},
};
use anyhow::{Context, Result, bail};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use tokio::sync::{broadcast, mpsc, oneshot, watch};

#[derive(Deserialize)]
pub struct Open {
    path: PathBuf,
    name: Option<String>,
    #[serde(default = "start_preview_default")]
    start_preview: bool,
}

fn start_preview_default() -> bool {
    true
}

#[derive(Deserialize)]
pub struct Selection {
    pub artboard: Option<String>,
    pub element: Option<String>,
}

pub enum Request {
    Projects,
    Preferences,
    SetTheme(String),
    Open(Open),
    State,
    Changes(u64),
    Edit(Vec<Edit>, Option<u64>, bool),
    Undo,
    Redo,
    Restore(u64),
    Selection(Selection),
}

pub enum Message {
    Request {
        request: Request,
        expected_project: Option<String>,
        reply: oneshot::Sender<Result<Value>>,
    },
    FilesChanged(notify::Result<()>),
    Shutdown(oneshot::Sender<Result<()>>),
}

#[derive(Clone)]
pub struct Event {
    pub name: &'static str,
    pub value: Value,
}

pub type Sender = mpsc::UnboundedSender<Message>;

struct ActiveProject {
    project: Project,
    loaded: LoadedDocuments,
    _watcher: RecommendedWatcher,
    snapshots: Vec<Snapshot>,
    current_snapshot: usize,
}

enum HistoryTarget {
    Previous,
    Next,
    Id(u64),
}

pub struct Actor {
    agent_dir: PathBuf,
    sender: Sender,
    receiver: mpsc::UnboundedReceiver<Message>,
    events: broadcast::Sender<Event>,
    active: Option<ActiveProject>,
    preview: Option<Preview>,
    selection: Value,
    revision: u64,
    changes: Vec<Value>,
    files_invalid: bool,
    shutdown: watch::Receiver<bool>,
}

impl Actor {
    pub fn new(
        agent_dir: PathBuf,
        sender: Sender,
        receiver: mpsc::UnboundedReceiver<Message>,
        events: broadcast::Sender<Event>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            agent_dir,
            sender,
            receiver,
            events,
            active: None,
            preview: None,
            selection: json!({"artboard": null, "element": null}),
            revision: 0,
            changes: Vec::new(),
            files_invalid: false,
            shutdown,
        }
    }

    pub async fn run(mut self) {
        let mut preview_health = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                message = self.receiver.recv() => match message {
                    Some(Message::Request { request, expected_project, reply }) => {
                        let result = self.request(request, expected_project.as_deref()).await;
                        if let Err(error) = &result {
                            self.emit_error(error);
                        }
                        let _ = reply.send(result);
                    }
                    Some(Message::FilesChanged(result)) => {
                        let result = result.map_err(anyhow::Error::from).and_then(|_| self.reload());
                        if let Err(error) = result {
                            self.emit_error(&error);
                        }
                    }
                    Some(Message::Shutdown(reply)) => {
                        let _ = reply.send(self.stop_preview().await);
                        break;
                    }
                    None => {
                        let _ = self.stop_preview().await;
                        break;
                    }
                },
                _ = preview_health.tick(), if self.preview.is_some() => {
                    let result = self.preview.as_mut().unwrap().exited();
                    match result {
                        Ok(Some(status)) => {
                            let _ = self.stop_preview().await;
                            self.emit_error(&anyhow::anyhow!("Vite exited: {status}"));
                            self.emit_state();
                        }
                        Err(error) => self.emit_error(&error),
                        Ok(None) => (),
                    }
                }
            }
        }
    }

    async fn request(&mut self, request: Request, expected_project: Option<&str>) -> Result<Value> {
        if let Some(expected) = expected_project {
            let actual = self
                .active
                .as_ref()
                .map(|active| active.project.id.as_str());
            if actual != Some(expected) {
                bail!(
                    "Active project changed; this Project refers to {expected}. Call pi_design.current()"
                );
            }
        }
        match request {
            Request::Projects => Ok(serde_json::to_value(registry::list(&self.agent_dir)?)?),
            Request::Preferences => {
                let path = self.agent_dir.join("design/preferences.json");
                if path.try_exists()? {
                    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
                } else {
                    Ok(json!({"theme": "dark"}))
                }
            }
            Request::SetTheme(theme) => {
                if theme != "dark" && theme != "light" {
                    bail!("Theme must be dark or light");
                }
                let preferences = json!({"theme": theme});
                documents::atomic_json(
                    &self.agent_dir.join("design/preferences.json"),
                    &preferences,
                )?;
                Ok(preferences)
            }
            Request::Open(open) => self.open(open).await,
            Request::State => {
                self.reload()?;
                Ok(self.state())
            }
            Request::Changes(since) => {
                self.reload()?;
                Ok(json!({
                    "revision": self.revision,
                    "changes": self.changes.iter().filter(|change| change["revision"].as_u64().unwrap() > since).collect::<Vec<_>>()
                }))
            }
            Request::Edit(edits, expected_revision, snapshot) => {
                self.reload()?;
                if let Some(expected) = expected_revision
                    && expected != self.revision
                {
                    bail!("Project changed since revision {expected}; read state and retry");
                }
                let active = self.active.as_ref().context("No active project")?;
                let mut next = active.loaded.documents.clone();
                next.edit(edits)?;
                documents::write_changed(
                    &PathBuf::from(&active.project.path),
                    &active.loaded,
                    &next,
                )?;
                self.reload_with_snapshot(snapshot)?;
                if snapshot && self.save_snapshot() {
                    self.emit_state();
                }
                Ok(self.state())
            }
            Request::Undo => self.travel_history(HistoryTarget::Previous),
            Request::Redo => self.travel_history(HistoryTarget::Next),
            Request::Restore(id) => self.travel_history(HistoryTarget::Id(id)),
            Request::Selection(selection) => {
                self.reload()?;
                self.active.as_ref().context("No active project")?;
                self.selection =
                    json!({"artboard": selection.artboard, "element": selection.element});
                self.emit_state();
                Ok(self.selection.clone())
            }
        }
    }

    async fn open(&mut self, open: Open) -> Result<Value> {
        let root = open
            .path
            .canonicalize()
            .with_context(|| format!("Open project {}", open.path.display()))?;
        if !root.is_dir() {
            bail!("Project path is not a directory: {}", root.display());
        }
        let default_name = open.name.clone().unwrap_or_else(|| {
            root.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Design".to_owned())
        });
        documents::create_missing(&root, &default_name)?;
        documents::read(&root)?;
        let project = registry::register(&self.agent_dir, &root, open.name.as_deref())?;
        let same_project = self
            .active
            .as_ref()
            .is_some_and(|active| active.project.id == project.id);
        if same_project {
            self.reload()?;
            self.active.as_mut().unwrap().project = project;
        } else {
            let sender = self.sender.clone();
            let design = root.join(documents::FILES[0]);
            let workspace = root.join(documents::FILES[1]);
            let directory = root.join(".design");
            let watched_directory = directory.clone();
            let mut watcher =
                notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                    match result {
                        Ok(event)
                            if !matches!(event.kind, EventKind::Access(_))
                                && event.paths.iter().any(|path| {
                                    path == &design
                                        || path == &workspace
                                        || path == &watched_directory
                                }) =>
                        {
                            let _ = sender.send(Message::FilesChanged(Ok(())));
                        }
                        Err(error) => {
                            let _ = sender.send(Message::FilesChanged(Err(error)));
                        }
                        _ => (),
                    }
                })?;
            watcher.watch(&root, RecursiveMode::NonRecursive)?;
            watcher.watch(&directory, RecursiveMode::NonRecursive)?;
            // Reload after installing the watcher so edits made while opening are not missed.
            let loaded = documents::read(&root)?;
            self.stop_preview().await?;
            let initial = Snapshot {
                id: self.revision,
                summary: "Opened project".to_owned(),
                documents: loaded.documents.clone(),
            };
            self.active = Some(ActiveProject {
                project,
                loaded,
                _watcher: watcher,
                snapshots: vec![initial],
                current_snapshot: 0,
            });
            self.changes.clear();
            self.files_invalid = false;
            self.selection = json!({"artboard": null, "element": null});
        }
        if !open.start_preview {
            self.stop_preview().await?;
        } else if self.preview.is_none() && root.join("package.json").try_exists()? {
            // Emit the selected project even if starting its Vite process fails.
            self.emit_state();
            self.preview = Some(Preview::start(&root, self.shutdown.clone()).await?);
        }
        self.emit_state();
        Ok(self.state())
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_with_snapshot(true)
    }

    fn reload_with_snapshot(&mut self, snapshot: bool) -> Result<()> {
        let Some(active) = self.active.as_ref() else {
            return Ok(());
        };
        let loaded = match documents::read(&PathBuf::from(&active.project.path)) {
            Ok(loaded) => loaded,
            Err(error) => {
                self.files_invalid = true;
                return Err(error);
            }
        };
        let recovered = std::mem::take(&mut self.files_invalid);
        if active.loaded.documents != loaded.documents {
            let previous = active.loaded.documents.clone();
            self.record_changes(&previous, &loaded.documents);
            self.active.as_mut().unwrap().loaded = loaded;
            if snapshot {
                self.save_snapshot();
            }
            self.emit_state();
        } else {
            // Formatting-only source edits still become the baseline for safe writes.
            self.active.as_mut().unwrap().loaded = loaded;
            if recovered {
                self.emit_state();
            }
        }
        Ok(())
    }

    fn save_snapshot(&mut self) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        let previous = &active.snapshots[active.current_snapshot].documents;
        let current = &active.loaded.documents;
        if previous == current {
            return false;
        }
        let snapshot = Snapshot {
            id: self.revision,
            summary: describe_change(previous, current),
            documents: current.clone(),
        };
        active.snapshots.truncate(active.current_snapshot + 1);
        active.snapshots.push(snapshot);
        active.current_snapshot = active.snapshots.len() - 1;
        true
    }

    fn record_changes(&mut self, previous: &Documents, next: &Documents) {
        for (index, file) in documents::FILES.iter().enumerate() {
            if previous.value(index) != next.value(index) {
                self.revision += 1;
                self.changes.push(json!({
                    "revision": self.revision,
                    "file": file,
                    "patch": json_patch::diff(previous.value(index), next.value(index))
                }));
            }
        }
    }

    fn travel_history(&mut self, target: HistoryTarget) -> Result<Value> {
        self.reload()?;
        self.save_snapshot();
        let active = self.active.as_ref().context("No active project")?;
        let index = match target {
            HistoryTarget::Previous => active.current_snapshot.saturating_sub(1),
            HistoryTarget::Next => (active.current_snapshot + 1).min(active.snapshots.len() - 1),
            HistoryTarget::Id(id) => active
                .snapshots
                .iter()
                .position(|snapshot| snapshot.id == id)
                .context("History snapshot is no longer available")?,
        };
        if index == active.current_snapshot {
            return Ok(self.state());
        }
        let next = active.snapshots[index].documents.clone();
        let previous = active.loaded.documents.clone();
        let root = PathBuf::from(&active.project.path);
        documents::write_changed(&root, &active.loaded, &next)?;
        let loaded = documents::read(&root)?;
        if loaded.documents != next {
            // A source editor wrote after our replacement. Its contents remain authoritative.
            self.reload()?;
            bail!("Project files changed during history navigation; read state and retry");
        }
        self.record_changes(&previous, &loaded.documents);
        let active = self.active.as_mut().unwrap();
        active.current_snapshot = index;
        active.loaded = loaded;
        self.emit_state();
        Ok(self.state())
    }

    fn state(&self) -> Value {
        let (project, design, workspace, can_undo, can_redo) = if let Some(active) = &self.active {
            (
                serde_json::to_value(&active.project).unwrap(),
                active.loaded.documents.design.clone(),
                active.loaded.documents.workspace.clone(),
                active.current_snapshot > 0,
                active.current_snapshot + 1 < active.snapshots.len(),
            )
        } else {
            (
                Value::Null,
                json!({"name": "", "elements": {}, "parameters": {}}),
                json!({"artboards": [], "comments": []}),
                false,
                false,
            )
        };
        let history = self.active.as_ref().map(|active| {
            json!({
                "current": active.snapshots[active.current_snapshot].id,
                "entries": active.snapshots.iter().map(|snapshot| json!({
                    "id": snapshot.id, "summary": snapshot.summary
                })).collect::<Vec<_>>()
            })
        });
        json!({
            "project": project, "revision": self.revision, "design": design, "workspace": workspace,
            "selection": self.selection, "preview_url": self.preview.as_ref().map(|preview| &preview.url),
            "can_undo": can_undo, "can_redo": can_redo, "history": history
        })
    }

    fn emit_state(&self) {
        let _ = self.events.send(Event {
            name: "state",
            value: self.state(),
        });
    }

    fn emit_error(&self, error: &anyhow::Error) {
        let _ = self.events.send(Event {
            name: "error",
            value: json!({"error": format!("{error:#}")}),
        });
    }

    async fn stop_preview(&mut self) -> Result<()> {
        if let Some(mut preview) = self.preview.take() {
            preview.stop().await?;
        }
        Ok(())
    }
}

pub async fn request(
    sender: &Sender,
    request: Request,
    expected_project: Option<String>,
) -> Result<Value> {
    let (reply, response) = oneshot::channel();
    sender
        .send(Message::Request {
            request,
            expected_project,
            reply,
        })
        .context("Design service stopped")?;
    response.await.context("Design service stopped")?
}
