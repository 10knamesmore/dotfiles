use std::{
    cell::{Cell, RefCell},
    env, fmt,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Stdio,
    rc::Rc,
};

use anyhow::Result;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::{mpsc, watch},
    task::JoinHandle,
};

#[derive(Debug)]
pub(super) struct CaptureError {
    pub code: &'static str,
    pub dependency: Option<&'static str>,
}

impl CaptureError {
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            dependency: None,
        }
    }

    pub fn dependency(code: &'static str, dependency: &'static str) -> Self {
        Self {
            code,
            dependency: Some(dependency),
        }
    }
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for CaptureError {}

pub(super) fn fail<T>(code: &'static str) -> Result<T> {
    Err(CaptureError::new(code).into())
}

pub(super) fn log(message: impl fmt::Display) {
    // Quickshell may close its diagnostic pipe while GSR is still finalizing.
    let _ = writeln!(io::stderr().lock(), "[capture] {message}");
}

#[derive(Clone)]
pub(super) struct Events(Rc<EventOutput>);

struct EventOutput {
    disconnected: Cell<bool>,
    shutdown: watch::Sender<bool>,
    lines: RefCell<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    writer: RefCell<Option<JoinHandle<()>>>,
}

impl Events {
    pub fn new() -> Self {
        let (shutdown, _) = watch::channel(false);
        let (lines, mut pending) = mpsc::unbounded_channel::<Vec<u8>>();
        let events = Self(Rc::new(EventOutput {
            disconnected: Cell::new(false),
            shutdown,
            lines: RefCell::new(Some(lines)),
            writer: RefCell::new(None),
        }));
        let output = events.clone();
        *events.0.writer.borrow_mut() = Some(tokio::task::spawn_local(async move {
            let mut stdout = tokio::io::stdout();
            while let Some(line) = pending.recv().await {
                let result = async {
                    stdout.write_all(&line).await?;
                    stdout.flush().await
                }
                .await;
                if let Err(error) = result {
                    output.0.disconnected.set(true);
                    output.shutdown();
                    if error.kind() != io::ErrorKind::BrokenPipe {
                        log(format_args!("protocol write failed: {error}"));
                    }
                    break;
                }
            }
        }));
        events
    }

    pub fn shutdown(&self) {
        self.0.shutdown.send_replace(true);
    }

    pub fn shutdown_receiver(&self) -> watch::Receiver<bool> {
        self.0.shutdown.subscribe()
    }

    pub fn emit(&self, event: Value) {
        if self.0.disconnected.get() {
            return;
        }
        let mut bytes = serde_json::to_vec(&event).expect("JSON value serialization");
        bytes.push(b'\n');
        if let Some(lines) = self.0.lines.borrow().as_ref() {
            let _ = lines.send(bytes);
        }
    }

    /// Drain protocol output after every producer has finished shutdown.
    pub async fn close(&self) {
        self.0.lines.borrow_mut().take();
        let writer = self.0.writer.borrow_mut().take();
        if let Some(writer) = writer
            && let Err(error) = writer.await
        {
            log(format_args!("protocol writer failed: {error}"));
        }
    }

    pub fn error(&self, operation: &str, error: &anyhow::Error, request_id: Option<&str>) {
        let capture = error.downcast_ref::<CaptureError>();
        let code = if let Some(error) = capture {
            error.code
        } else if error.is::<serde_json::Error>()
            || error.is::<std::num::ParseIntError>()
            || error.is::<std::str::Utf8Error>()
            || error.is::<env::VarError>()
        {
            "invalid_command"
        } else if error.is::<io::Error>() {
            "io_failed"
        } else {
            "operation_failed"
        };
        log(format_args!("operation={operation} code={code}: {error:#}"));
        let mut event = json!({"type":"error", "operation":operation, "code":code});
        if let Some(request_id) = request_id {
            event["requestId"] = json!(request_id);
        }
        if let Some(dependency) = capture.and_then(|error| error.dependency) {
            event["dependency"] = json!(dependency);
        }
        self.emit(event);
    }
}

pub(super) async fn cancelled(receiver: &mut watch::Receiver<bool>) {
    if !*receiver.borrow() {
        let _ = receiver.wait_for(|value| *value).await;
    }
}

pub(super) fn session_directory() -> Result<PathBuf> {
    Ok(PathBuf::from(env::var("XDG_RUNTIME_DIR")?)
        .join("hypr")
        .join(env::var("HYPRLAND_INSTANCE_SIGNATURE")?))
}

pub(super) fn socket_path() -> Result<PathBuf> {
    Ok(session_directory()?.join("dots-capture.sock"))
}

pub(super) fn executable_exists(name: &str) -> bool {
    env::split_paths(&env::var_os("PATH").unwrap_or_default()).any(|directory| {
        let path = directory.join(name);
        std::fs::metadata(&path).is_ok_and(|metadata| metadata.is_file())
            && nix::unistd::access(&path, nix::unistd::AccessFlags::X_OK).is_ok()
    })
}

pub(super) fn spawn_error(program: &'static str, error: io::Error) -> anyhow::Error {
    if error.kind() == io::ErrorKind::NotFound {
        CaptureError::dependency("missing_dependency", program).into()
    } else {
        error.into()
    }
}

pub(super) async fn run_command(program: &'static str, args: &[&str]) -> Result<Vec<u8>> {
    run_command_cancellable(program, args, None).await
}

/// A cancelled metadata request kills and reaps each of its compositor queries.
pub(super) async fn run_command_cancellable(
    program: &'static str,
    args: &[&str],
    mut cancellation: Option<watch::Receiver<bool>>,
) -> Result<Vec<u8>> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| spawn_error(program, error))?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let mut output = Vec::new();
    let mut diagnostic = Vec::new();
    let completion = {
        let read = async {
            tokio::try_join!(
                stdout.read_to_end(&mut output),
                stderr.read_to_end(&mut diagnostic),
                child.wait(),
            )
        };
        tokio::pin!(read);
        tokio::select! {
            result = &mut read => Some(result),
            () = async {
                match cancellation.as_mut() {
                    Some(receiver) => cancelled(receiver).await,
                    None => std::future::pending().await,
                }
            } => None,
        }
    };
    let Some(completion) = completion else {
        child.kill().await?;
        return fail("request_cancelled");
    };
    let status = match completion {
        Ok((_, _, status)) => status,
        Err(error) => {
            let _ = child.kill().await;
            return Err(error.into());
        }
    };
    if !status.success() {
        log(format_args!("{program} exited {status}"));
        return fail("command_failed");
    }
    Ok(output)
}

pub(super) async fn hyprland_json(query: &str) -> Result<Value> {
    Ok(serde_json::from_slice(
        &run_command("hyprctl", &["-j", query]).await?,
    )?)
}

pub(super) async fn copy_to_clipboard(mime_type: &str, data: &[u8]) -> Result<()> {
    // The daemon owns the selection beyond server shutdown; only its parent is
    // awaited here. Its inherited stderr must not keep a diagnostic pipe open.
    let diagnostic = tempfile::tempfile()?;
    let mut child = Command::new("wl-copy")
        .args(["--type", mime_type])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(diagnostic)
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| spawn_error("wl-copy", error))?;
    let input = async {
        let mut stdin = child.stdin.take().expect("piped stdin");
        stdin.write_all(data).await?;
        stdin.shutdown().await
    }
    .await;
    if let Err(error) = input {
        let _ = child.kill().await;
        return Err(error.into());
    }
    let status = child.wait().await?;
    if !status.success() {
        log(format_args!("wl-copy exited {status}"));
        return fail("command_failed");
    }
    Ok(())
}

pub(super) async fn export_directory(kind: &str, folder: &str) -> Result<PathBuf> {
    let output = run_command("xdg-user-dir", &[kind]).await?;
    let directory = PathBuf::from(std::str::from_utf8(&output)?.trim());
    if !directory.is_absolute() {
        return fail("invalid_save_directory");
    }
    let destination = directory.join(folder);
    tokio::fs::create_dir_all(&destination).await?;
    Ok(destination)
}

pub(super) fn export_filename(prefix: &str, extension: &str) -> String {
    format!(
        "{prefix}_{}_{}.{extension}",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"),
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

pub(super) fn string_field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CaptureError::new("invalid_command").into())
}

pub(super) fn integer_field(value: &Value, key: &str, code: &'static str) -> Result<i64> {
    value
        .get(key)
        .ok_or_else(|| CaptureError::new("invalid_command"))?
        .as_i64()
        .ok_or_else(|| CaptureError::new(code).into())
}

pub(super) fn remove_file(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
