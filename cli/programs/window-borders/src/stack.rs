//! Read yabai's focused stack after its state-change signals, off the UI thread.

use std::{
    process::Command,
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq)]
pub struct FocusedStack {
    pub window: u32,
    /// Window IDs in ascending yabai stack-index order.
    pub members: Vec<u32>,
}

#[derive(Deserialize, PartialEq)]
struct WindowFrame {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct StackWindow {
    id: u32,
    frame: WindowFrame,
    stack_index: u32,
    has_focus: bool,
}

type StackQueryResult = Result<Option<FocusedStack>>;

fn query_focused_stack() -> StackQueryResult {
    let output = Command::new("yabai")
        .args([
            "-m",
            "query",
            "--windows",
            "id,frame,stack-index,has-focus",
            "--space",
        ])
        .output()
        .context("query yabai stack")?;
    ensure!(
        output.status.success(),
        "yabai stack query failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let windows: Vec<StackWindow> =
        serde_json::from_slice(&output.stdout).context("decode yabai stack windows")?;
    let Some(focused) = windows
        .iter()
        .find(|window| window.has_focus && window.stack_index > 0)
    else {
        return Ok(None);
    };
    // yabai exposes an index, but no stack ID. Group indexed windows with the
    // same frame in the focused Space.
    let mut members = windows
        .iter()
        .filter(|window| window.stack_index > 0 && window.frame == focused.frame)
        .collect::<Vec<_>>();
    members.sort_unstable_by_key(|window| window.stack_index);
    Ok((members.len() > 1).then_some(FocusedStack {
        window: focused.id,
        members: members.into_iter().map(|window| window.id).collect(),
    }))
}

pub struct StackReader {
    requests: Option<SyncSender<()>>,
    latest: Arc<Mutex<Option<StackQueryResult>>>,
    worker: Option<JoinHandle<()>>,
}

impl StackReader {
    pub fn new(wake: fn()) -> Result<Self> {
        let (requests, receiver) = mpsc::sync_channel(1);
        let latest = Arc::new(Mutex::new(None));
        let results = Arc::clone(&latest);
        let worker = thread::Builder::new()
            .name("yabai-stack-query".into())
            .spawn(move || {
                for () in receiver {
                    *results.lock().unwrap() = Some(query_focused_stack());
                    wake();
                }
            })?;
        let reader = Self {
            requests: Some(requests),
            latest,
            worker: Some(worker),
        };
        reader.request_update()?;
        Ok(reader)
    }

    /// Keep one pending refresh while a query is in flight; bursts need only
    /// the newest yabai snapshot, not one subprocess per notification.
    pub fn request_update(&self) -> Result<()> {
        match self.requests.as_ref().unwrap().try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()),
            Err(TrySendError::Disconnected(())) => anyhow::bail!("yabai stack worker stopped"),
        }
    }

    pub fn take_update(&self) -> Option<StackQueryResult> {
        self.latest.lock().unwrap().take()
    }
}

impl Drop for StackReader {
    fn drop(&mut self) {
        // Close the producer before joining so an idle worker can exit.
        self.requests.take();
        self.worker.take().unwrap().join().unwrap();
    }
}
