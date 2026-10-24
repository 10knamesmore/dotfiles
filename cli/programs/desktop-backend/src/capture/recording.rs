use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::Result;
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{Notify, watch},
    task::JoinHandle,
};

use super::common::{
    CaptureError, Events, cancelled, executable_exists, export_directory, export_filename, fail,
    log, spawn_error,
};
use super::portal::PortalMonitor;

pub(super) type CancelPicker = Box<dyn Fn(Option<&str>)>;

#[derive(Clone)]
pub(super) struct Recording(Rc<RecorderState>);

struct RecorderState {
    events: Events,
    monitor: PortalMonitor,
    active: RefCell<Option<Rc<Session>>>,
    cancel_picker: RefCell<Option<CancelPicker>>,
}

struct Session {
    options: AudioAndCursor,
    control: RefCell<Control>,
    wake: Notify,
    done: watch::Sender<bool>,
    task: RefCell<Option<JoinHandle<()>>>,
}

struct AudioAndCursor {
    system_audio: bool,
    microphone: bool,
    cursor: bool,
}

#[derive(Default)]
struct Control {
    pid: Option<u32>,
    launched: bool,
    picker_id: Option<String>,
    window_address: Option<u64>,
    /// Epoch seconds from GSR's first-frame sidecar, never from process startup.
    started_at: Option<f64>,
    selection_time: Option<Instant>,
    stop_requested: bool,
    cancelled: bool,
    failure: Option<&'static str>,
}

impl Session {
    fn request_stop(&self) {
        let mut control = self.control.borrow_mut();
        if !control.stop_requested {
            control.cancelled = control.started_at.is_none();
            control.stop_requested = true;
            self.wake.notify_one();
        }
    }
}

impl Recording {
    pub fn new(events: Events, monitor: PortalMonitor) -> Self {
        Self(Rc::new(RecorderState {
            events,
            monitor,
            active: RefCell::new(None),
            cancel_picker: RefCell::new(None),
        }))
    }

    pub fn set_picker_canceller(&self, callback: CancelPicker) {
        *self.0.cancel_picker.borrow_mut() = Some(callback);
    }

    pub fn prepare(&self, command: &Value) -> Result<()> {
        if self.0.active.borrow().is_some() {
            return fail("recording_in_progress");
        }
        if !executable_exists("gpu-screen-recorder") {
            self.emit_state("error");
            return Err(
                CaptureError::dependency("missing_dependency", "gpu-screen-recorder").into(),
            );
        }
        if !self.0.monitor.available() {
            self.emit_state("error");
            return Err(CaptureError::dependency("portal_monitor_unavailable", "busctl").into());
        }
        let boolean = |key: &str| {
            command
                .get(key)
                .and_then(Value::as_bool)
                .ok_or_else(|| CaptureError::new("invalid_command"))
        };
        let options = AudioAndCursor {
            system_audio: boolean("systemAudio")?,
            microphone: boolean("microphone")?,
            cursor: boolean("cursor")?,
        };
        let (done, _) = watch::channel(false);
        let session = Rc::new(Session {
            options,
            control: RefCell::new(Control::default()),
            wake: Notify::new(),
            done,
            task: RefCell::new(None),
        });
        *self.0.active.borrow_mut() = Some(session.clone());
        let recorder = self.clone();
        let running = session.clone();
        *session.task.borrow_mut() = Some(tokio::task::spawn_local(async move {
            recorder.run(running).await;
        }));
        Ok(())
    }

    pub fn picker_opened(&self, request_id: &str, caller_pid: Option<u32>) {
        let session = self.0.active.borrow().clone();
        if let Some(session) = session {
            let mut control = session.control.borrow_mut();
            if control.pid.is_some() && caller_pid == control.pid {
                control.picker_id = Some(request_id.to_owned());
                session.wake.notify_one();
            }
        }
    }

    pub fn picker_selected(&self, request_id: &str, address: Option<&str>) -> Result<()> {
        let session = self.0.active.borrow().clone();
        if let Some(session) = session {
            let mut control = session.control.borrow_mut();
            if control.picker_id.as_deref() == Some(request_id) {
                if control.stop_requested {
                    return fail("recording_cancelled");
                }
                control.window_address = address
                    .map(|address| u64::from_str_radix(address.trim_start_matches("0x"), 16))
                    .transpose()?
                    .filter(|address| *address != 0);
                control.selection_time = Some(Instant::now());
                session.wake.notify_one();
                self.emit_state("starting");
            }
        }
        Ok(())
    }

    pub fn picker_cancelled(&self, request_id: &str) {
        let session = self.0.active.borrow().clone();
        if let Some(session) = session {
            let owns_picker = session.control.borrow().picker_id.as_deref() == Some(request_id);
            if owns_picker {
                session.request_stop();
            }
        }
    }

    pub fn window_closed(&self, address: &str) -> Result<()> {
        let address = u64::from_str_radix(address.trim_start_matches("0x"), 16)?;
        let session = self.0.active.borrow().clone();
        if let Some(session) = session
            && session.control.borrow().window_address == Some(address)
        {
            session.request_stop();
        }
        Ok(())
    }

    /// Concurrent callers wait on completion, without borrowing the child or its task.
    pub async fn stop(&self) {
        let session = self.0.active.borrow().clone();
        if let Some(session) = session {
            let mut done = session.done.subscribe();
            session.request_stop();
            cancelled(&mut done).await;
            let task = session.task.borrow_mut().take();
            if let Some(task) = task
                && let Err(error) = task.await
            {
                log(format_args!("recording task failed: {error}"));
            }
        }
    }

    fn emit_state(&self, state: &str) {
        self.0
            .events
            .emit(json!({"type":"recording", "state":state}));
    }

    fn cancel_picker(&self, session: &Session) {
        let request_id = session.control.borrow().picker_id.clone();
        if let Some(cancel) = self.0.cancel_picker.borrow().as_ref() {
            cancel(request_id.as_deref());
        }
    }

    async fn run(&self, session: Rc<Session>) {
        let mut path = None;
        let result = self.prepare_and_record(&session, &mut path).await;
        if let Err(error) = result {
            if let Some(path) = &path
                && let Err(error) = discard(path).await
            {
                log(format_args!("discard recording failed: {error}"));
            }
            self.emit_state("error");
            let operation = if session.control.borrow().launched {
                "recording"
            } else {
                "record_prepare"
            };
            self.0.events.error(operation, &error, None);
        }
        if let Some(path) = &path
            && let Err(error) = discard(&timestamp_path(path)).await
        {
            self.0.events.error("recording", &error.into(), None);
        }
        self.cancel_picker(&session);
        self.0.active.borrow_mut().take();
        session.done.send_replace(true);
    }

    async fn prepare_and_record(
        &self,
        session: &Session,
        path: &mut Option<PathBuf>,
    ) -> Result<()> {
        let destination = export_directory("VIDEOS", "Recordings")
            .await?
            .join(export_filename("Recording", "mp4"));
        *path = Some(destination.clone());
        if session.control.borrow().stop_requested {
            self.emit_state("idle");
            return Ok(());
        }
        let mut command = Command::new("gpu-screen-recorder");
        command
            .args([
                "-w",
                "portal",
                "-restore-portal-session",
                "no",
                "-k",
                "h264",
                "-ac",
                "aac",
                "-c",
                "mp4",
                "-f",
                "60",
                "-cursor",
                if session.options.cursor { "yes" } else { "no" },
                "-write-first-frame-ts",
                "yes",
                "-o",
            ])
            .arg(&destination);
        let mut audio = Vec::new();
        if session.options.system_audio {
            audio.push("default_output");
        }
        if session.options.microphone {
            audio.push("default_input");
        }
        if !audio.is_empty() {
            command.args(["-a", &audio.join("|")]);
        }
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| spawn_error("gpu-screen-recorder", error))?;
        {
            let mut control = session.control.borrow_mut();
            control.pid = child.id();
            control.launched = true;
        }
        let stderr = child.stderr.take().expect("piped stderr");
        let diagnostics = tokio::task::spawn_local(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line).await {
                    Ok(0) => break,
                    Ok(_) => log(format_args!(
                        "gpu-screen-recorder: {}",
                        String::from_utf8_lossy(&line).trim_end()
                    )),
                    Err(error) => {
                        log(format_args!("recorder diagnostic read failed: {error}"));
                        break;
                    }
                }
            }
        });
        self.emit_state("selecting");
        let result = self.watch(session, &destination, &mut child).await;
        if result.is_err() {
            let _ = child.kill().await;
        }
        session.control.borrow_mut().pid = None;
        diagnostics.await?;
        let status = result?;
        self.finish_video(session, &destination, status).await
    }

    async fn finish_video(&self, session: &Session, path: &Path, status: ExitStatus) -> Result<()> {
        let needs_first_frame = {
            let control = session.control.borrow();
            control.started_at.is_none() && !control.cancelled
        };
        let first_frame = if needs_first_frame {
            first_frame_time(path).await?
        } else {
            None
        };
        let (failure, cancelled, started_at) = {
            let mut control = session.control.borrow_mut();
            if control.started_at.is_none()
                && !control.cancelled
                && let Some(started_at) = first_frame
            {
                if control.selection_time.is_none() {
                    return fail("picker_not_identified");
                }
                control.started_at = Some(started_at);
            }
            (control.failure, control.cancelled, control.started_at)
        };
        if let Some(failure) = failure {
            return fail(failure);
        }
        if cancelled {
            discard(path).await?;
            self.emit_state("idle");
        } else if !status.success() {
            return fail("recorder_exited");
        } else if started_at.is_none() {
            return fail("first_frame_missing");
        } else {
            let metadata = match tokio::fs::metadata(path).await {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return fail("empty_recording");
                }
                Err(error) => return Err(error.into()),
            };
            if metadata.len() == 0 {
                return fail("empty_recording");
            }
            self.0
                .events
                .emit(json!({"type":"recording", "state":"idle", "path":path}));
            log(format_args!("recording saved bytes={}", metadata.len()));
        }
        Ok(())
    }

    async fn watch(&self, session: &Session, path: &Path, child: &mut Child) -> Result<ExitStatus> {
        let launched_at = Instant::now();
        let mut stop_time = None;
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            let now = Instant::now();
            let check_first_frame = {
                let control = session.control.borrow();
                !control.stop_requested && control.started_at.is_none()
            };
            let first_frame = if check_first_frame {
                first_frame_time(path).await?
            } else {
                None
            };
            {
                let mut control = session.control.borrow_mut();
                if !control.stop_requested && control.started_at.is_none() {
                    if let Some(started_at) = first_frame {
                        if control.selection_time.is_none() {
                            control.failure = Some("picker_not_identified");
                        } else {
                            control.started_at = Some(started_at);
                            self.0.events.emit(json!({"type":"recording", "state":"recording", "startedAt":started_at}));
                        }
                    } else if control.selection_time.is_some_and(|selected| {
                        now.duration_since(selected) > Duration::from_secs(15)
                    }) {
                        control.failure = Some("first_frame_timeout");
                    } else if control.picker_id.is_none()
                        && now.duration_since(launched_at) > Duration::from_secs(30)
                    {
                        control.failure = Some("picker_not_started");
                    }
                    if control.failure.is_some() {
                        drop(control);
                        session.request_stop();
                    }
                }
            }
            let stop_requested = session.control.borrow().stop_requested;
            if stop_requested && stop_time.is_none() {
                self.cancel_picker(session);
                if let Some(pid) = child.id() {
                    kill(Pid::from_raw(pid as i32), Signal::SIGINT)
                        .map_err(std::io::Error::from)?;
                }
                stop_time = Some(now);
                if !session.control.borrow().cancelled {
                    self.emit_state("saving");
                }
            }
            if let Some(stopped) = stop_time {
                let timeout = if session.control.borrow().cancelled {
                    10
                } else {
                    30
                };
                if now.duration_since(stopped) > Duration::from_secs(timeout) {
                    session.control.borrow_mut().failure = Some("recorder_stop_timeout");
                    child.kill().await?;
                    return Ok(child.wait().await?);
                }
            }
            tokio::select! {
                status = child.wait() => return Ok(status?),
                _ = session.wake.notified() => {}
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
        }
    }
}

fn timestamp_path(path: &Path) -> PathBuf {
    let mut filename = path.as_os_str().to_os_string();
    filename.push(".ts");
    PathBuf::from(filename)
}

async fn discard(path: &Path) -> std::io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

async fn first_frame_time(path: &Path) -> Result<Option<f64>> {
    let content = match tokio::fs::read_to_string(timestamp_path(path)).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !content.ends_with('\n') {
        return Ok(None);
    }
    let Some(line) = content.lines().nth(1) else {
        return Ok(None);
    };
    let values: Vec<&str> = line.split_whitespace().collect();
    if values.len() != 2
        || values
            .iter()
            .any(|value| value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Ok(None);
    }
    Ok(Some(values[1].parse::<u64>()? as f64 / 1_000_000.0))
}
