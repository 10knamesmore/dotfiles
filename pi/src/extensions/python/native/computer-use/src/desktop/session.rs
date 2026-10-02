//! One execution thread per desktop; XKB and Wayland state never cross threads.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use pyo3::prelude::*;

use super::background::Background;
use super::control::{ACTIVE, CLOSED, Shared};
use super::{Control, Target};
use crate::{input::Input, logging, wait};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static DESKTOPS: OnceLock<Mutex<Vec<Weak<Handle>>>> = OnceLock::new();

pub(super) enum Open {
    Host(Target),
    Background { python: String, size: (u32, u32) },
}

type Action = Box<dyn FnOnce(&mut Session) + Send>;
struct Job {
    cancel: Arc<AtomicBool>,
    action: Action,
}

pub(super) struct Handle {
    pub id: u64,
    pub shared: Arc<Shared>,
    sender: mpsc::Sender<Job>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Handle {
    pub fn open(py: Python<'_>, options: Open) -> PyResult<Arc<Self>> {
        let (sender, receiver) = mpsc::channel::<Job>();
        let (ready_sender, ready) = mpsc::sync_channel(1);
        let shared = Arc::new(Shared {
            status: ACTIVE.into(),
        });
        let control_shared = shared.clone();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let thread = std::thread::Builder::new()
            .name(format!("computer-use-{id}"))
            .spawn(move || {
                let mut control = Control::new(control_shared);
                let opened = (|| {
                    let (background, target) = match options {
                        Open::Host(target) => {
                            logging::result("host.connect", control.connect_host(&target))?;
                            (None, target)
                        }
                        Open::Background { python, size } => {
                            let (background, target) = Background::start(&control, &python, size)?;
                            (Some(background), target)
                        }
                    };
                    let input = Input::new(&control, &target)?;
                    Ok((background, target, input))
                })();
                let (background, target, input) = match opened {
                    Ok(opened) => opened,
                    Err(error) => {
                        control.disconnect();
                        control.close();
                        let _ = ready_sender.send(Err(error));
                        return;
                    }
                };
                let mut session = Session {
                    control,
                    target,
                    input: Some(input),
                    background,
                };
                if ready_sender.send(Ok(())).is_ok() {
                    loop {
                        if session.control.check().is_err() {
                            break;
                        }
                        if let Some(background) = session.background.as_mut()
                            && !background.alive().unwrap_or(false)
                        {
                            session.control.disconnect();
                            break;
                        }
                        match receiver.recv_timeout(Duration::from_millis(20)) {
                            Ok(job) => {
                                *session.control.cancel.borrow_mut() = Some(job.cancel);
                                (job.action)(&mut session);
                                *session.control.cancel.borrow_mut() = None;
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                }
                session.control.shared.invalidate(CLOSED);
            })
            .map_err(wait::runtime)?;
        let handle = Arc::new(Self {
            id,
            shared,
            sender,
            thread: Mutex::new(Some(thread)),
        });
        let opened = handle.receive(py, ready, None);
        if let Err(error) = opened {
            handle.close(py);
            return Err(error);
        }
        DESKTOPS
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .push(Arc::downgrade(&handle));
        Ok(handle)
    }

    fn receive<T: Send>(
        &self,
        py: Python<'_>,
        receiver: mpsc::Receiver<PyResult<T>>,
        cancel: Option<&AtomicBool>,
    ) -> PyResult<T> {
        let receiver = Mutex::new(receiver);
        loop {
            if let Err(error) = py.check_signals() {
                if let Some(cancel) = cancel {
                    cancel.store(true, Ordering::Release);
                }
                self.close(py);
                return Err(error);
            }
            match py.detach(|| {
                receiver
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_millis(20))
            }) {
                Ok(result) => {
                    if let Err(error) = py.check_signals() {
                        if let Some(cancel) = cancel {
                            cancel.store(true, Ordering::Release);
                        }
                        self.close(py);
                        return Err(error);
                    }
                    return result;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.shared.check()?;
                    self.shared.invalidate(super::control::CLOSED);
                    return Err(wait::runtime("desktop execution thread stopped"));
                }
            }
        }
    }

    pub fn call<T: Send + 'static>(
        &self,
        py: Python<'_>,
        operation: &'static str,
        action: impl FnOnce(&mut Session) -> PyResult<T> + Send + 'static,
    ) -> PyResult<T> {
        self.shared.check()?;
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel(1);
        self.sender
            .send(Job {
                cancel: cancel.clone(),
                action: Box::new(move |session| {
                    let result = session
                        .control
                        .check()
                        .and_then(|()| action(session))
                        .and_then(|value| session.control.check().map(|()| value));
                    let _ = sender.send(logging::result(operation, result));
                }),
            })
            .map_err(|_| wait::runtime(format!("desktop is {}", self.shared.status())))?;
        let result = self.receive(py, receiver, Some(&cancel));
        if self.shared.check().is_err() {
            self.join(py);
        }
        result
    }

    pub fn close(&self, py: Python<'_>) {
        self.shared.invalidate(CLOSED);
        self.join(py);
    }

    fn join(&self, py: Python<'_>) {
        py.detach(|| {
            let mut thread = self.thread.lock().unwrap();
            if let Some(thread) = thread.take()
                && thread.join().is_err()
            {
                logging::event("desktop.close", "thread-panicked");
            }
        });
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.shared.invalidate(CLOSED);
        // The execution thread owns cleanup; dropping the Python object never drops XKB on this thread.
        // Explicit close and atexit join it; implicit collection lets its next poll finish cleanup.
    }
}

pub(super) struct Session {
    pub control: Control,
    pub target: Target,
    input: Option<Input>,
    background: Option<Background>,
}

impl Session {
    pub fn input<T>(
        &mut self,
        action: impl FnOnce(&mut Input, &Control, &Target) -> PyResult<T>,
    ) -> PyResult<T> {
        action(self.input.as_mut().unwrap(), &self.control, &self.target)
    }

    pub fn release_inputs(&mut self) -> PyResult<()> {
        let result = self.input.as_mut().unwrap().release_all(&self.control);
        self.control
            .event(serde_json::json!({"type":"keys", "keys":[]}))?;
        result
    }

    pub fn launch(&mut self, argv: Vec<String>, cwd: Option<String>) -> PyResult<u32> {
        if argv.is_empty() || argv[0].is_empty() {
            return Err(wait::invalid("argv must contain an executable"));
        }
        if let Some(background) = self.background.as_mut() {
            return background.launch(&self.control, argv, cwd);
        }
        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .env("XDG_RUNTIME_DIR", &self.target.runtime)
            .env("WAYLAND_DISPLAY", &self.target.display)
            .env("HYPRLAND_INSTANCE_SIGNATURE", &self.target.signature)
            .env_remove("WAYLAND_SOCKET")
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let mut child = command.spawn().map_err(wait::runtime)?;
        let pid = child.id();
        // Host apps belong to the user after launch; reap them without coupling their life to close().
        std::thread::Builder::new()
            .name("computer-use-host-app".into())
            .spawn(move || {
                let _ = child.wait();
            })
            .map_err(wait::runtime)?;
        Ok(pid)
    }

    fn cleanup(&mut self) {
        if let Some(mut input) = self.input.take() {
            let _ = input.release_all(&self.control);
            drop(input);
        }
        self.control.close();
        self.background.take();
        logging::event("desktop.close", "finished");
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.cleanup();
    }
}

pub(super) fn active_handles() -> Vec<Arc<Handle>> {
    let mut desktops = DESKTOPS.get_or_init(Default::default).lock().unwrap();
    let handles: Vec<_> = desktops.iter().filter_map(Weak::upgrade).collect();
    desktops.retain(|desktop| desktop.strong_count() > 0);
    handles
}
