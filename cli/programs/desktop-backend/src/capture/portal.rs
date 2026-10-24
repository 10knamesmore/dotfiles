use std::{
    cell::{Cell, RefCell},
    process::Stdio,
    rc::Rc,
    time::Duration,
};

use anyhow::Result;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    process::Command,
    sync::{Notify, watch},
    task::JoinHandle,
};

use super::{
    common::{
        CaptureError, Events, cancelled, fail, hyprland_json, integer_field, log,
        run_command_cancellable, spawn_error, string_field,
    },
    recording::{CancelPicker, Recording},
    screenshots::Monitor,
};

const PORTAL_SERVICE: &str = "org.freedesktop.impl.portal.desktop.hyprland";
const PORTAL_INTERFACE: &str = "org.freedesktop.impl.portal.ScreenCast";

struct PortalCall {
    sender: String,
    cookie: u64,
    caller_pid: Cell<Option<u32>>,
    resolved: Cell<bool>,
}

#[derive(Clone)]
pub(super) struct PortalMonitor(Rc<MonitorState>);

struct MonitorState {
    events: Events,
    available: Cell<bool>,
    observing: Cell<bool>,
    seen: Cell<bool>,
    changed: Notify,
    calls: RefCell<Vec<Rc<PortalCall>>>,
    stop: watch::Sender<bool>,
    task: RefCell<Option<JoinHandle<()>>>,
}

impl PortalMonitor {
    pub fn new(events: Events) -> Self {
        let (stop, _) = watch::channel(false);
        Self(Rc::new(MonitorState {
            events,
            available: Cell::new(false),
            observing: Cell::new(false),
            seen: Cell::new(false),
            changed: Notify::new(),
            calls: RefCell::new(Vec::new()),
            stop,
            task: RefCell::new(None),
        }))
    }

    pub fn available(&self) -> bool {
        self.0.available.get()
    }

    pub async fn start(&self) {
        if let Err(error) = self.start_observation().await {
            log(format_args!("portal monitor could not start: {error:#}"));
            self.0.events.error(
                "picker",
                &CaptureError::dependency("portal_monitor_unavailable", "busctl").into(),
                None,
            );
            self.close().await;
        }
    }

    async fn start_observation(&self) -> Result<()> {
        let mut child = Command::new("busctl")
            .args([
                "--user",
                "--json=short",
                "--no-pager",
                "monitor",
                PORTAL_SERVICE,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| spawn_error("busctl", error))?;
        let stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        let monitor = self.clone();
        let mut stop = self.0.stop.subscribe();
        self.0.observing.set(true);
        *self.0.task.borrow_mut() = Some(tokio::task::spawn_local(async move {
            let observation = async {
                let mut lines = BufReader::new(stdout).lines();
                let result: Result<()> = loop {
                    tokio::select! {
                        () = cancelled(&mut stop) => break Ok(()),
                        line = lines.next_line() => {
                            match line {
                                Ok(Some(line)) => {
                                    if let Err(error) = monitor.observe(&line).await { break Err(error); }
                                }
                                Ok(None) => {
                                    break fail("portal_monitor_failed");
                                }
                                Err(error) => break Err(error.into()),
                            }
                        }
                    }
                };
                monitor.0.available.set(false);
                monitor.0.observing.set(false);
                monitor.0.changed.notify_waiters();
                if let Err(error) = child.kill().await {
                    log(format_args!("portal monitor cleanup failed: {error}"));
                }
                if !*monitor.0.stop.borrow()
                    && let Err(error) = result
                {
                    monitor.0.events.error("picker", &error, None);
                }
            };
            // Drain diagnostics throughout observation, including after UI disconnect.
            let drain = async { tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await };
            let (_, diagnostic) = tokio::join!(observation, drain);
            if let Err(error) = diagnostic {
                log(format_args!("portal diagnostic read failed: {error}"));
            }
        }));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while !self.0.seen.get() {
            let (cancel_probe, cancellation) = watch::channel(false);
            let arguments = [
                "--user",
                "--timeout=1",
                "get-property",
                PORTAL_SERVICE,
                "/org/freedesktop/portal/desktop",
                PORTAL_INTERFACE,
                "version",
            ];
            let probe = run_command_cancellable("busctl", &arguments, Some(cancellation));
            tokio::pin!(probe);
            match tokio::time::timeout_at(deadline, &mut probe).await {
                Ok(result) => {
                    result?;
                }
                Err(error) => {
                    cancel_probe.send_replace(true);
                    let _ = probe.await;
                    return Err(error.into());
                }
            }
            if !self.0.seen.get() {
                tokio::time::timeout_at(deadline, tokio::time::sleep(Duration::from_millis(100)))
                    .await?;
            }
        }
        if !self.0.observing.get() {
            return fail("portal_monitor_failed");
        }
        self.0.available.set(true);
        log("portal request monitor ready");
        Ok(())
    }

    async fn observe(&self, line: &str) -> Result<()> {
        let message: Value = serde_json::from_str(line)?;
        if message["interface"] == "org.freedesktop.DBus.Properties" && message["member"] == "Get" {
            self.0.seen.set(true);
        }
        if message["type"] == "method_call"
            && message["interface"] == PORTAL_INTERFACE
            && message["member"] == "SelectSources"
        {
            let call = Rc::new(PortalCall {
                sender: string_field(&message, "sender")?.to_owned(),
                cookie: message["cookie"]
                    .as_u64()
                    .ok_or_else(|| CaptureError::new("invalid_command"))?,
                caller_pid: Cell::new(None),
                resolved: Cell::new(false),
            });
            self.0.calls.borrow_mut().push(call.clone());
            self.0.changed.notify_waiters();
            let lookup = async {
                let session_handle = message["payload"]["data"][1]
                    .as_str()
                    .ok_or_else(|| CaptureError::new("invalid_command"))?;
                // session/<original unique bus name>/<token> identifies the app.
                // The forwarded method's sender identifies xdg-desktop-portal instead.
                let original = session_handle
                    .rsplit('/')
                    .nth(1)
                    .ok_or_else(|| CaptureError::new("invalid_command"))?;
                let sender = format!(":{}", original.replace('_', "."));
                let response = run_command_cancellable(
                    "busctl",
                    &[
                        "--user",
                        "--json=short",
                        "--timeout=2",
                        "call",
                        "org.freedesktop.DBus",
                        "/org/freedesktop/DBus",
                        "org.freedesktop.DBus",
                        "GetConnectionUnixProcessID",
                        "s",
                        &sender,
                    ],
                    Some(self.0.stop.subscribe()),
                )
                .await?;
                let response: Value = serde_json::from_slice(&response)?;
                let pid = response["data"][0]
                    .as_u64()
                    .and_then(|pid| u32::try_from(pid).ok())
                    .ok_or_else(|| CaptureError::new("invalid_command"))?;
                Ok::<u32, anyhow::Error>(pid)
            }
            .await;
            match lookup {
                Ok(pid) => call.caller_pid.set(Some(pid)),
                Err(error) => log(format_args!("portal caller lookup failed: {error:#}")),
            }
            call.resolved.set(true);
            self.0.changed.notify_waiters();
        } else if message["type"] == "method_return" || message["type"] == "error" {
            self.0.calls.borrow_mut().retain(|call| {
                message["destination"].as_str() != Some(call.sender.as_str())
                    || message["reply_cookie"].as_u64() != Some(call.cookie)
            });
            self.0.changed.notify_waiters();
        }
        Ok(())
    }

    pub async fn caller_pid(&self) -> Option<u32> {
        tokio::time::timeout(Duration::from_secs(2), async {
            while self.available() {
                let changed = self.0.changed.notified();
                let call = self.0.calls.borrow().first().cloned();
                if let Some(call) = call
                    && call.resolved.get()
                {
                    return call.caller_pid.get();
                }
                changed.await;
            }
            None
        })
        .await
        .unwrap_or(None)
    }

    pub async fn close(&self) {
        self.0.available.set(false);
        self.0.stop.send_replace(true);
        self.0.changed.notify_waiters();
        let task = self.0.task.borrow_mut().take();
        if let Some(task) = task
            && let Err(error) = task.await
        {
            log(format_args!("portal monitor task failed: {error}"));
        }
        self.0.calls.borrow_mut().clear();
    }
}

struct PickerRequest {
    request_id: String,
    windows: Vec<Value>,
    result: watch::Sender<Option<Value>>,
}

#[derive(Clone)]
pub(super) struct PickerBridge(Rc<PickerState>);

struct PickerState {
    events: Events,
    monitor: PortalMonitor,
    recording: Recording,
    active: RefCell<Option<Rc<PickerRequest>>>,
    closing: Cell<bool>,
}

impl PickerBridge {
    pub fn new(events: Events, monitor: PortalMonitor, recording: Recording) -> Self {
        Self(Rc::new(PickerState {
            events,
            monitor,
            recording,
            active: RefCell::new(None),
            closing: Cell::new(false),
        }))
    }

    pub fn recording_canceller(&self) -> CancelPicker {
        let picker = Rc::downgrade(&self.0);
        Box::new(move |request_id| {
            if let Some(picker) = picker.upgrade() {
                Self(picker).cancel_recording_picker(request_id);
            }
        })
    }

    pub async fn connect(&self, connection: UnixStream) {
        let mut reader = BufReader::new(connection);
        let mut picker = None;
        let result = self.handle_connection(&mut reader, &mut picker).await;
        if let Err(error) = result {
            let disconnected = error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                )
            });
            if !disconnected {
                self.0.events.error(
                    "picker",
                    &error,
                    picker.as_ref().map(|picker| picker.request_id.as_str()),
                );
            }
        }
        if let Some(picker) = picker {
            self.finish(&picker, None, false);
        }
        let _ = reader.get_mut().shutdown().await;
    }

    async fn handle_connection(
        &self,
        reader: &mut BufReader<UnixStream>,
        active_connection: &mut Option<Rc<PickerRequest>>,
    ) -> Result<()> {
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line)).await??;
        let request: Value = serde_json::from_str(&line)?;
        let windows = request["windows"]
            .as_array()
            .ok_or_else(|| CaptureError::new("invalid_request"))?;
        if request["type"] != "picker" {
            return fail("invalid_request");
        }
        if self.0.active.borrow().is_some() || self.0.closing.get() {
            reader
                .get_mut()
                .write_all(b"{\"cancelled\":true}\n")
                .await?;
            return Ok(());
        }
        if windows.iter().any(|window| {
            ["id", "address", "title", "appId"]
                .iter()
                .any(|key| !window[*key].is_string())
        }) {
            return fail("invalid_request");
        }
        let (result, mut response) = watch::channel(None);
        let picker = Rc::new(PickerRequest {
            request_id: uuid::Uuid::new_v4().simple().to_string(),
            windows: windows.clone(),
            result,
        });
        *self.0.active.borrow_mut() = Some(picker.clone());
        *active_connection = Some(picker.clone());
        let caller_pid = self.0.monitor.caller_pid().await;
        // Shutdown may have rejected this connection while caller lookup was pending.
        if self.is_active(&picker) {
            self.0
                .recording
                .picker_opened(&picker.request_id, caller_pid);
            self.0.events.emit(json!({
                "type":"picker_request", "requestId":picker.request_id, "windows":picker.windows,
            }));
        }
        let mut extra = Vec::new();
        tokio::select! {
            _ = reader.read_to_end(&mut extra) => {
                self.finish(&picker, None, false);
                return Ok(());
            }
            _ = response.wait_for(|response| response.is_some()) => {}
        }
        let result = if self.is_active(&picker) {
            response.borrow().clone().expect("picker result is ready")
        } else {
            json!({"cancelled":true})
        };
        let mut bytes = serde_json::to_vec(&result)?;
        bytes.push(b'\n');
        reader.get_mut().write_all(&bytes).await?;
        if result["cancelled"] == false {
            self.finish(&picker, Some(result), true);
        } else {
            self.finish(&picker, None, true);
        }
        Ok(())
    }

    fn is_active(&self, picker: &Rc<PickerRequest>) -> bool {
        self.0
            .active
            .borrow()
            .as_ref()
            .is_some_and(|active| Rc::ptr_eq(active, picker))
    }

    fn request(&self, request_id: &str) -> Result<Rc<PickerRequest>> {
        self.0
            .active
            .borrow()
            .as_ref()
            .filter(|picker| picker.request_id == request_id)
            .cloned()
            .ok_or_else(|| CaptureError::new("unknown_picker").into())
    }

    pub async fn select(&self, request_id: &str, selection: &Value) -> Result<()> {
        let picker = self.request(request_id)?;
        if picker.result.borrow().is_some() {
            return fail("selection_in_progress");
        }
        let kind = string_field(selection, "type")?;
        let mut address = None;
        let text = match kind {
            "window" => {
                let token = string_field(selection, "id")?;
                let window = picker
                    .windows
                    .iter()
                    .find(|window| window["id"] == token)
                    .ok_or_else(|| CaptureError::new("invalid_selection"))?;
                address = Some(string_field(window, "address")?.to_owned());
                format!("[SELECTION]/window:{token}")
            }
            "output" | "region" => {
                let output = string_field(selection, "output")?;
                let monitors: Vec<Monitor> =
                    serde_json::from_value(hyprland_json("monitors").await?)?;
                let monitor = monitors
                    .iter()
                    .find(|monitor| monitor.name == output && !monitor.disabled)
                    .ok_or_else(|| CaptureError::new("invalid_selection"))?;
                if kind == "output" {
                    format!("[SELECTION]/screen:{}", monitor.name)
                } else {
                    let x = integer_field(selection, "x", "invalid_selection")?;
                    let y = integer_field(selection, "y", "invalid_selection")?;
                    let width = integer_field(selection, "width", "invalid_selection")?;
                    let height = integer_field(selection, "height", "invalid_selection")?;
                    let (monitor_width, monitor_height) = monitor.rotated_size();
                    if x < 0
                        || y < 0
                        || width <= 0
                        || height <= 0
                        || x.checked_add(width)
                            .is_none_or(|right| right as f64 > monitor_width as f64 / monitor.scale)
                        || y.checked_add(height).is_none_or(|bottom| {
                            bottom as f64 > monitor_height as f64 / monitor.scale
                        })
                    {
                        return fail("invalid_selection");
                    }
                    format!(
                        "[SELECTION]/region:{}@{x},{y},{width},{height}",
                        monitor.name
                    )
                }
            }
            _ => return fail("invalid_selection"),
        };
        if !self.is_active(&picker) {
            return fail("unknown_picker");
        }
        if picker.result.borrow().is_some() {
            return fail("selection_in_progress");
        }
        self.0
            .recording
            .picker_selected(request_id, address.as_deref())?;
        self.finish(
            &picker,
            Some(json!({"cancelled":false, "selection":text})),
            false,
        );
        Ok(())
    }

    pub fn cancel(&self, request_id: &str) -> Result<()> {
        self.finish(&self.request(request_id)?, None, false);
        Ok(())
    }

    fn finish(&self, picker: &Rc<PickerRequest>, result: Option<Value>, delivered: bool) {
        if !self.is_active(picker) {
            return;
        }
        if picker.result.borrow().is_none() {
            picker.result.send_replace(Some(
                result.clone().unwrap_or_else(|| json!({"cancelled":true})),
            ));
        }
        // Keep the request active until the socket accepted its response. A
        // disconnect before delivery must still cancel an associated recording.
        if result.is_some() && !delivered {
            return;
        }
        self.0.active.borrow_mut().take();
        let cancelled = result.is_none();
        self.0.events.emit(
            json!({"type":"picker_closed", "requestId":picker.request_id, "cancelled":cancelled}),
        );
        if cancelled {
            self.0.recording.picker_cancelled(&picker.request_id);
        }
    }

    pub fn cancel_recording_picker(&self, request_id: Option<&str>) {
        let picker = self.0.active.borrow().clone();
        if let Some(picker) = picker
            && Some(picker.request_id.as_str()) == request_id
        {
            self.finish(&picker, None, false);
        }
    }

    pub fn close(&self) {
        self.0.closing.set(true);
        let picker = self.0.active.borrow().clone();
        if let Some(picker) = picker {
            self.finish(&picker, None, false);
        }
    }
}
