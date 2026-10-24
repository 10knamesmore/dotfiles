use std::{
    fs::File,
    io::{self, Read},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll, ready},
};

use anyhow::Result;
use nix::{
    errno::Errno,
    fcntl::{Flock, FlockArg, OFlag},
    sys::stat::{Mode, umask},
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader, ReadBuf, unix::AsyncFd},
    net::{UnixListener, UnixStream},
    signal::unix::{SignalKind, signal},
    task::{JoinHandle, JoinSet},
};

use super::{
    common::{
        CaptureError, Events, cancelled, executable_exists, fail, log, remove_file,
        session_directory, socket_path, string_field,
    },
    portal::{PickerBridge, PortalMonitor},
    recording::Recording,
    screenshots::Screenshots,
};

/// Field order keeps the lock held through temporary-directory removal.
struct SessionFiles {
    root: TempDir,
    socket: PathBuf,
    _lock: Flock<File>,
}

impl SessionFiles {
    fn acquire() -> Result<Self> {
        let socket = socket_path()?;
        let mut lock_path = socket.as_os_str().to_os_string();
        lock_path.push(".lock");
        let lock = File::options()
            .append(true)
            .create(true)
            .open(PathBuf::from(lock_path))?;
        let lock = Flock::lock(lock, FlockArg::LockExclusiveNonblock).map_err(|(_, error)| {
            if error == Errno::EWOULDBLOCK {
                anyhow::Error::from(CaptureError::new("server_already_running"))
            } else {
                io::Error::from(error).into()
            }
        })?;
        // Only the lock owner may replace the previous session's socket.
        remove_file(&socket)?;
        let root = tempfile::Builder::new()
            .prefix("dots-capture-")
            .tempdir_in(session_directory()?)?;
        Ok(Self {
            root,
            socket,
            _lock: lock,
        })
    }
}

impl Drop for SessionFiles {
    fn drop(&mut self) {
        if let Err(error) = remove_file(&self.socket) {
            log(format_args!("socket cleanup failed: {error}"));
        }
    }
}

#[derive(Clone)]
struct CaptureServer {
    events: Events,
    screenshots: Screenshots,
    monitor: PortalMonitor,
    recording: Recording,
    picker: PickerBridge,
}

pub async fn serve() -> Result<()> {
    umask(Mode::from_bits_truncate(0o077));
    let events = Events::new();
    let result = run(events.clone()).await;
    if let Err(error) = &result {
        events.error("startup", error, None);
    }
    events.close().await;
    result
}

async fn run(events: Events) -> Result<()> {
    let files = SessionFiles::acquire()?;
    let monitor = PortalMonitor::new(events.clone());
    let recording = Recording::new(events.clone(), monitor.clone());
    let picker = PickerBridge::new(events.clone(), monitor.clone(), recording.clone());
    recording.set_picker_canceller(picker.recording_canceller());
    let server = CaptureServer {
        screenshots: Screenshots::new(files.root.path().to_owned(), events.clone()),
        monitor,
        recording,
        picker,
        events: events.clone(),
    };
    let mut jobs = JoinSet::new();
    let mut connections = JoinSet::new();
    let mut windows = None;
    let result = server
        .listen(&files.socket, &mut jobs, &mut connections, &mut windows)
        .await;
    log("capture server stopping");
    events.shutdown();
    server.picker.close();
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            log(format_args!("picker connection task failed: {error}"));
        }
    }
    // A stop command can already be awaiting this same recording. Neither
    // waiter owns a lock needed by the recording or the picker's cancellation.
    server.recording.stop().await;
    while let Some(result) = jobs.join_next().await {
        if let Err(error) = result {
            log(format_args!("capture command task failed: {error}"));
        }
    }
    server.screenshots.close().await;
    if let Some(windows) = windows
        && let Err(error) = windows.await
    {
        log(format_args!("window watcher task failed: {error}"));
    }
    server.monitor.close().await;
    drop(files);
    log("capture server stopped");
    result
}

impl CaptureServer {
    async fn listen(
        &self,
        socket: &PathBuf,
        jobs: &mut JoinSet<()>,
        connections: &mut JoinSet<()>,
        windows: &mut Option<JoinHandle<()>>,
    ) -> Result<()> {
        let mut terminate = signal(SignalKind::terminate())?;
        let mut interrupt = signal(SignalKind::interrupt())?;
        self.monitor.start().await;
        let listener = UnixListener::bind(socket)?;
        std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))?;
        let input = ProtocolInput::open()?;
        let mut lines = BufReader::new(input).lines();
        let watcher = self.clone();
        *windows = Some(tokio::task::spawn_local(async move {
            watcher.watch_windows().await
        }));
        let mut shutdown = self.events.shutdown_receiver();
        self.events.emit(json!({
            "type":"ready",
            "dependencies":{
                "wlCopy":executable_exists("wl-copy"),
                "gpuScreenRecorder":executable_exists("gpu-screen-recorder"),
            },
        }));
        log("capture server ready");
        loop {
            tokio::select! {
                () = cancelled(&mut shutdown) => break,
                _ = terminate.recv() => break,
                _ = interrupt.recv() => break,
                line = lines.next_line() => {
                    let Some(line) = line? else { break; };
                    self.read_command(&line, jobs);
                }
                connection = listener.accept() => {
                    let (connection, _) = connection?;
                    let picker = self.picker.clone();
                    connections.spawn_local(async move { picker.connect(connection).await });
                }
                Some(result) = jobs.join_next(), if !jobs.is_empty() => {
                    if let Err(error) = result {
                        log(format_args!("capture command task failed: {error}"));
                    }
                }
                Some(result) = connections.join_next(), if !connections.is_empty() => {
                    if let Err(error) = result {
                        log(format_args!("picker connection task failed: {error}"));
                    }
                }
            }
        }
        Ok(())
    }

    fn read_command(&self, line: &str, jobs: &mut JoinSet<()>) {
        let command: Value = match serde_json::from_str(line) {
            Ok(command) => command,
            Err(error) => {
                self.events.error("command", &error.into(), None);
                return;
            }
        };
        if let Err(error) = self.dispatch(&command, jobs) {
            self.events.error(
                command
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("command"),
                &error,
                command.get("requestId").and_then(Value::as_str),
            );
        }
    }

    fn dispatch(&self, command: &Value, jobs: &mut JoinSet<()>) -> Result<()> {
        if !command.is_object() {
            return fail("invalid_command");
        }
        let operation = string_field(command, "type")?;
        let request_id = command.get("requestId").and_then(Value::as_str);
        if matches!(
            operation,
            "snapshot" | "release" | "export" | "picker_select" | "picker_cancel"
        ) && request_id.is_none_or(str::is_empty)
        {
            return fail("invalid_command");
        }
        match operation {
            "snapshot" => self
                .screenshots
                .snapshot(request_id.expect("validated request ID"))?,
            "record_prepare" => self.recording.prepare(command)?,
            "release" | "export" | "copy_color" | "picker_select" | "picker_cancel"
            | "stop_recording" => {
                let server = self.clone();
                let command = command.clone();
                jobs.spawn_local(async move {
                    let operation = command["type"].as_str().expect("validated operation");
                    if let Err(error) = server.execute(&command).await {
                        server.events.error(
                            operation,
                            &error,
                            command.get("requestId").and_then(Value::as_str),
                        );
                    }
                });
            }
            _ => return fail("unknown_command"),
        }
        Ok(())
    }

    async fn execute(&self, command: &Value) -> Result<()> {
        match string_field(command, "type")? {
            "release" => {
                self.screenshots
                    .release(string_field(command, "requestId")?)
                    .await?
            }
            "export" => self.screenshots.export(command).await?,
            "copy_color" => {
                self.screenshots
                    .copy_color(string_field(command, "value")?)
                    .await?
            }
            "picker_select" => {
                self.picker
                    .select(
                        string_field(command, "requestId")?,
                        command
                            .get("selection")
                            .ok_or_else(|| CaptureError::new("invalid_command"))?,
                    )
                    .await?
            }
            "picker_cancel" => self.picker.cancel(string_field(command, "requestId")?)?,
            "stop_recording" => self.recording.stop().await,
            _ => unreachable!("dispatch admits only asynchronous commands"),
        }
        Ok(())
    }

    async fn watch_windows(&self) {
        let result = async {
            let socket = session_directory()?.join(".socket2.sock");
            let mut shutdown = self.events.shutdown_receiver();
            let connection = tokio::select! {
                () = cancelled(&mut shutdown) => return Ok(()),
                result = UnixStream::connect(socket) => result?,
            };
            let mut lines = BufReader::new(connection).lines();
            loop {
                tokio::select! {
                    () = cancelled(&mut shutdown) => return Ok(()),
                    line = lines.next_line() => {
                        let Some(line) = line? else {
                            self.events.shutdown();
                            return Ok(());
                        };
                        if let Some(address) = line.trim().strip_prefix("closewindow>>") {
                            self.recording.window_closed(address)?;
                        }
                    }
                }
            }
        }
        .await;
        if let Err(error) = result {
            self.events.error("recording", &error, None);
        }
    }
}

/// AsyncFd avoids Tokio stdin's uncancellable blocking read during signal shutdown.
struct ProtocolInput(AsyncFd<File>);

impl ProtocolInput {
    #[allow(unsafe_code)]
    fn open() -> io::Result<Self> {
        let file = File::options()
            .read(true)
            .custom_flags(OFlag::O_NONBLOCK.bits())
            .open("/dev/stdin")?;
        // SAFETY: File owns this descriptor and never replaces or closes it while registered.
        Ok(Self(unsafe { AsyncFd::register(file) }?))
    }
}

impl AsyncRead for ProtocolInput {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut ready = ready!(self.0.poll_read_ready(context))?;
            match ready.try_io(|fd| {
                let mut file = fd.get_ref();
                file.read(buffer.initialize_unfilled())
            }) {
                Ok(Ok(count)) => {
                    buffer.advance(count);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(error)) => return Poll::Ready(Err(error)),
                Err(_) => continue,
            }
        }
    }
}
