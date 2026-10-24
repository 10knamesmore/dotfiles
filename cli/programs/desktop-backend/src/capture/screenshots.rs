use std::{
    cell::RefCell,
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
    rc::Rc,
};

use anyhow::Result;
use image::{
    ImageEncoder, ImageFormat, ImageReader,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::{sync::watch, task::JoinHandle};

use super::common::{
    CaptureError, Events, copy_to_clipboard, export_directory, export_filename, fail,
    integer_field, log, run_command_cancellable, string_field,
};

#[derive(Clone)]
pub(super) struct Screenshots(Rc<ScreenshotSessions>);

struct ScreenshotSessions {
    root: PathBuf,
    events: Events,
    snapshots: RefCell<FxHashMap<String, Rc<Snapshot>>>,
}

struct Snapshot {
    directory: TempDir,
    cancel: watch::Sender<bool>,
    task: RefCell<Option<JoinHandle<()>>>,
}

impl Screenshots {
    pub fn new(root: PathBuf, events: Events) -> Self {
        Self(Rc::new(ScreenshotSessions {
            root,
            events,
            snapshots: RefCell::new(FxHashMap::default()),
        }))
    }

    pub fn snapshot(&self, request_id: &str) -> Result<()> {
        if self.0.snapshots.borrow().contains_key(request_id) {
            return fail("request_in_use");
        }
        let (cancel, _) = watch::channel(false);
        let snapshot = Rc::new(Snapshot {
            directory: tempfile::Builder::new()
                .prefix("snapshot-")
                .tempdir_in(&self.0.root)?,
            cancel,
            task: RefCell::new(None),
        });
        self.0
            .snapshots
            .borrow_mut()
            .insert(request_id.to_owned(), snapshot.clone());
        let screenshots = self.clone();
        let request_id = request_id.to_owned();
        let preparing = snapshot.clone();
        *snapshot.task.borrow_mut() = Some(tokio::task::spawn_local(async move {
            let result = screenshots.prepare(&request_id, &preparing).await;
            if let Err(error) = result
                && !*preparing.cancel.borrow()
            {
                screenshots
                    .0
                    .events
                    .error("snapshot", &error, Some(&request_id));
                screenshots.0.snapshots.borrow_mut().remove(&request_id);
            }
        }));
        Ok(())
    }

    async fn prepare(&self, request_id: &str, snapshot: &Snapshot) -> Result<()> {
        let query = |query: &'static str| async move {
            run_command_cancellable("hyprctl", &["-j", query], Some(snapshot.cancel.subscribe()))
                .await
        };
        // Join all four queries even on failure, so every child is reaped.
        let (monitors, clients, pointer, focused) = tokio::join!(
            query("monitors"),
            query("clients"),
            query("cursorpos"),
            query("activewindow"),
        );
        if *snapshot.cancel.borrow() {
            return Ok(());
        }
        let monitors: Vec<Monitor> = serde_json::from_slice(&monitors?)?;
        let clients: Vec<Client> = serde_json::from_slice(&clients?)?;
        let pointer: Value = serde_json::from_slice(&pointer?)?;
        let focused: Value = serde_json::from_slice(&focused?)?;
        let screens: Vec<Value> = monitors
            .iter()
            .enumerate()
            .filter(|(_, monitor)| !monitor.disabled)
            .map(|(index, monitor)| {
                let (width, height) = monitor.rotated_size();
                json!({
                    "name": monitor.name, "x":monitor.x, "y":monitor.y,
                    "width":width as f64 / monitor.scale, "height":height as f64 / monitor.scale,
                    "pixelWidth":width, "pixelHeight":height, "scale":monitor.scale,
                    "exportPath":snapshot.directory.path().join(format!("display-{index}.bmp")),
                    "activeWorkspace":monitor.active_workspace.id,
                })
            })
            .collect();
        if screens.is_empty() {
            return fail("no_outputs");
        }
        let windows: Vec<Value> = clients.iter().filter(|client| client.mapped && !client.hidden)
            .map(|client| json!({
                "address":client.address, "title":client.title, "appId":client.class,
                "x":client.at[0], "y":client.at[1], "width":client.size[0], "height":client.size[1],
                "monitor":monitors.iter().find(|monitor| monitor.id == client.monitor).map(|monitor| monitor.name.as_str()).unwrap_or(""),
                "workspaceId":client.workspace.id,
                "focused":focused["address"].as_str() == Some(client.address.as_str()),
            })).collect();
        self.0.events.emit(json!({
            "type":"snapshot", "requestId":request_id, "screens":screens, "windows":windows, "pointer":pointer,
        }));
        log(format_args!(
            "snapshot metadata completed displays={} windows={}",
            screens.len(),
            windows.len()
        ));
        Ok(())
    }

    pub async fn release(&self, request_id: &str) -> Result<()> {
        let snapshot = self.0.snapshots.borrow_mut().remove(request_id);
        if let Some(snapshot) = snapshot {
            snapshot.cancel.send_replace(true);
            let task = snapshot.task.borrow_mut().take();
            if let Some(task) = task {
                task.await?;
            }
            // Export jobs can still hold a reference; release removes files now.
            match tokio::fs::remove_dir_all(snapshot.directory.path()).await {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                result => result?,
            }
        }
        Ok(())
    }

    pub async fn export(&self, command: &Value) -> Result<()> {
        let request_id = string_field(command, "requestId")?;
        let snapshot = self
            .0
            .snapshots
            .borrow()
            .get(request_id)
            .cloned()
            .ok_or_else(|| CaptureError::new("unknown_request"))?;
        let action = string_field(command, "action")?;
        if !matches!(action, "copy" | "save") {
            return fail("invalid_action");
        }
        let image_path = PathBuf::from(string_field(command, "imagePath")?);
        if !image_path.is_absolute() {
            return fail("invalid_image");
        }
        let image_path = tokio::fs::canonicalize(image_path)
            .await
            .map_err(|_| CaptureError::new("invalid_image"))?;
        let snapshot_path = tokio::fs::canonicalize(snapshot.directory.path()).await?;
        if image_path.parent() != Some(snapshot_path.as_path()) {
            return fail("invalid_image");
        }
        let crop = command
            .get("crop")
            .cloned()
            .ok_or_else(|| CaptureError::new("invalid_command"))?;
        let png = tokio::task::spawn_blocking(move || crop_to_png(image_path, crop)).await??;
        if action == "copy" {
            copy_to_clipboard("image/png", &png).await?;
            self.0
                .events
                .emit(json!({"type":"exported", "requestId":request_id, "action":action}));
        } else {
            let path = export_directory("PICTURES", "Screenshots")
                .await?
                .join(export_filename("Screenshot", "png"));
            let destination = path.clone();
            tokio::task::spawn_blocking(move || -> Result<()> {
                let mut output = File::options()
                    .write(true)
                    .create_new(true)
                    .open(&destination)?;
                if let Err(error) = output.write_all(&png) {
                    super::common::remove_file(&destination)?;
                    return Err(error.into());
                }
                Ok(())
            })
            .await??;
            self.0.events.emit(
                json!({"type":"exported", "requestId":request_id, "action":action, "path":path}),
            );
        }
        Ok(())
    }

    pub async fn copy_color(&self, value: &str) -> Result<()> {
        let expression = regex_lite::Regex::new(
            r"^(#[0-9a-fA-F]{6}|rgb\(\s*[0-9]{1,3}\s*,\s*[0-9]{1,3}\s*,\s*[0-9]{1,3}\s*\))$",
        )?;
        if !expression.is_match(value) {
            return fail("invalid_color");
        }
        if let Some(components) = value
            .strip_prefix("rgb(")
            .and_then(|rest| rest.strip_suffix(')'))
            && components
                .split(',')
                .any(|part| part.trim().parse::<u16>().map_or(true, |part| part > 255))
        {
            return fail("invalid_color");
        }
        copy_to_clipboard("text/plain;charset=utf-8", value.as_bytes()).await?;
        self.0
            .events
            .emit(json!({"type":"copied_color", "value":value}));
        Ok(())
    }

    pub async fn close(&self) {
        let request_ids: Vec<String> = self.0.snapshots.borrow().keys().cloned().collect();
        for request_id in request_ids {
            if let Err(error) = self.release(&request_id).await {
                self.0.events.error("release", &error, Some(&request_id));
            }
        }
    }
}

fn crop_to_png(path: PathBuf, crop: Value) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|_| CaptureError::new("invalid_image"))?;
    let reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|_| CaptureError::new("invalid_image"))?;
    if reader.format() != Some(ImageFormat::Bmp) {
        return fail("invalid_image");
    }
    let image = reader
        .decode()
        .map_err(|_| CaptureError::new("invalid_image"))?;
    let x = integer_field(&crop, "x", "invalid_crop")?;
    let y = integer_field(&crop, "y", "invalid_crop")?;
    let width = integer_field(&crop, "width", "invalid_crop")?;
    let height = integer_field(&crop, "height", "invalid_crop")?;
    if x < 0
        || y < 0
        || width <= 0
        || height <= 0
        || x.checked_add(width)
            .is_none_or(|right| right > image.width() as i64)
        || y.checked_add(height)
            .is_none_or(|bottom| bottom > image.height() as i64)
    {
        return fail("invalid_crop");
    }
    let cropped = image
        .crop_imm(x as u32, y as u32, width as u32, height as u32)
        .into_rgba8();
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            cropped.as_raw(),
            cropped.width(),
            cropped.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|_| CaptureError::new("invalid_image"))?;
    Ok(png)
}

#[derive(Deserialize)]
pub(super) struct Monitor {
    pub id: i64,
    pub name: String,
    pub x: i64,
    pub y: i64,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub transform: u32,
    #[serde(default)]
    pub disabled: bool,
    #[serde(rename = "activeWorkspace")]
    active_workspace: Workspace,
}

impl Monitor {
    pub fn rotated_size(&self) -> (u32, u32) {
        if !self.transform.is_multiple_of(2) {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        }
    }
}

#[derive(Deserialize)]
struct Workspace {
    id: i64,
}

#[derive(Deserialize)]
struct Client {
    address: String,
    title: String,
    class: String,
    at: [i64; 2],
    size: [i64; 2],
    monitor: i64,
    workspace: Workspace,
    mapped: bool,
    hidden: bool,
}
