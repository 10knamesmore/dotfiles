//! Notification callbacks collect IDs; the main thread owns all window state.

use std::{
    collections::BTreeSet,
    ffi::c_void,
    ptr::null_mut,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result};
use dispatch2::DispatchQueue;
use objc2_core_foundation::{
    CFMachPort, CFMachPortContext, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopDefaultMode,
};
use objc2_core_graphics::{
    CGDisplayChangeSummaryFlags, CGDisplayRegisterReconfigurationCallback,
    CGDisplayRemoveReconfigurationCallback,
};
use signal_hook::{
    consts::{SIGINT, SIGTERM, SIGUSR1},
    iterator::{Handle, Signals},
};

use super::{Connection, check, ffi::*, owned};

#[derive(Default)]
pub struct Changes {
    pub created: BTreeSet<WindowId>,
    pub changed: BTreeSet<WindowId>,
    pub closed: BTreeSet<WindowId>,
    pub focus: bool,
    pub spaces: bool,
    pub displays: bool,
    pub stack_refresh: bool,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.created.is_empty()
            && self.changed.is_empty()
            && self.closed.is_empty()
            && !self.focus
            && !self.spaces
            && !self.displays
            && !self.stack_refresh
    }
}

// No callbacks borrow manager state. SkyLight registrations last until exit.
static PENDING: LazyLock<Mutex<Changes>> = LazyLock::new(|| Mutex::new(Changes::default()));
static EXIT: AtomicBool = AtomicBool::new(false);

pub struct Events {
    port: CFRetained<CFMachPort>,
    source: CFRetained<CFRunLoopSource>,
    run_loop: CFRetained<CFRunLoop>,
    signals: Handle,
    signal_worker: Option<JoinHandle<()>>,
}

impl Events {
    pub fn new(connection: Connection) -> Result<Self> {
        let mut signals = Signals::new([SIGINT, SIGTERM, SIGUSR1])?;
        let signal_handle = signals.handle();
        let signal_worker =
            thread::Builder::new()
                .name("border-signals".into())
                .spawn(move || {
                    for signal in signals.forever() {
                        match signal {
                            SIGUSR1 => PENDING.lock().unwrap().stack_refresh = true,
                            SIGINT | SIGTERM => EXIT.store(true, Ordering::Relaxed),
                            _ => unreachable!(),
                        }
                        wake_main_loop();
                    }
                })?;
        unsafe {
            for event in [723, 804, 806, 807, 808, 811, 815, 816, 1322] {
                check(
                    SLSRegisterNotifyProc(window_changed, event, null_mut()),
                    "register window event",
                )?;
            }
            for event in [1325, 1326] {
                check(
                    SLSRegisterNotifyProc(window_membership_changed, event, null_mut()),
                    "register window membership event",
                )?;
            }
            for event in [1401, 1508] {
                check(
                    SLSRegisterNotifyProc(global_changed, event, null_mut()),
                    "register global event",
                )?;
            }
            let mut port_number = 0;
            check(
                SLSGetEventPort(connection.id, &mut port_number),
                "get WindowServer event port",
            )?;
            let mut context = CFMachPortContext {
                version: 0,
                info: null_mut(),
                retain: None,
                release: None,
                copyDescription: None,
            };
            let port = CFMachPort::with_port(
                None,
                port_number,
                Some(drain_events),
                &mut context,
                null_mut(),
            )
            .context("create WindowServer Mach port")?;
            _CFMachPortSetOptions(&port, 0x40);
            let source = CFMachPort::new_run_loop_source(None, Some(&port), 0)
                .context("create WindowServer event source")?;
            let run_loop = CFRunLoop::main().context("get main run loop")?;
            run_loop.add_source(Some(&source), kCFRunLoopDefaultMode);
            check(
                CGDisplayRegisterReconfigurationCallback(Some(display_changed), null_mut()).0,
                "watch display configuration",
            )?;
            Ok(Self {
                port,
                source,
                run_loop,
                signals: signal_handle,
                signal_worker: Some(signal_worker),
            })
        }
    }

    pub fn wait(&self, seconds: f64) {
        unsafe {
            CFRunLoop::run_in_mode(kCFRunLoopDefaultMode, seconds, true);
        }
    }

    pub fn take_changes(&self) -> Changes {
        std::mem::take(&mut *PENDING.lock().unwrap())
    }

    pub fn should_exit(&self) -> bool {
        EXIT.load(Ordering::Relaxed)
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        self.signals.close();
        self.signal_worker.take().unwrap().join().unwrap();
        unsafe {
            CGDisplayRemoveReconfigurationCallback(Some(display_changed), null_mut());
            self.run_loop
                .remove_source(Some(&self.source), kCFRunLoopDefaultMode);
        }
        self.source.invalidate();
        self.port.invalidate();
    }
}

pub(crate) fn wake_main_loop() {
    DispatchQueue::main().exec_async(|| CFRunLoop::main().unwrap().stop());
}

unsafe extern "C" fn window_changed(event: u32, data: *const c_void, _: usize, _: *mut c_void) {
    let window = unsafe { data.cast::<u32>().read_unaligned() };
    let mut changes = PENDING.lock().unwrap();
    if event == 804 {
        changes.closed.insert(window);
    } else {
        changes.changed.insert(window);
    }
    if matches!(event, 723 | 804 | 808 | 815 | 816 | 1322) {
        changes.focus = true;
    }
}

unsafe extern "C" fn window_membership_changed(
    event: u32,
    data: *const c_void,
    _: usize,
    _: *mut c_void,
) {
    // Payload: Space ID (u64), window ID (u32). Destroy refers to membership
    // in that Space, not necessarily to the lifetime of the window.
    let window = unsafe { data.cast::<u8>().add(8).cast::<u32>().read_unaligned() };
    let mut changes = PENDING.lock().unwrap();
    if event == 1325 {
        changes.created.insert(window);
    } else {
        changes.changed.insert(window);
    }
    changes.focus = true;
}

unsafe extern "C" fn global_changed(event: u32, _: *const c_void, _: usize, _: *mut c_void) {
    let mut changes = PENDING.lock().unwrap();
    changes.focus = true;
    changes.spaces |= event == 1401;
}

unsafe extern "C-unwind" fn display_changed(
    _: u32,
    flags: CGDisplayChangeSummaryFlags,
    _: *mut c_void,
) {
    if flags.contains(CGDisplayChangeSummaryFlags::BeginConfigurationFlag) {
        return;
    }
    let already_pending = {
        let mut changes = PENDING.lock().unwrap();
        std::mem::replace(&mut changes.displays, true)
    };
    if !already_pending {
        wake_main_loop();
    }
}

unsafe extern "C-unwind" fn drain_events(
    _: *mut CFMachPort,
    _: *mut c_void,
    _: isize,
    _: *mut c_void,
) {
    let connection = Connection::main();
    loop {
        let event = unsafe { SLEventCreateNextEvent(connection.id) };
        if event.is_null() {
            break;
        }
        drop(unsafe { owned(event, "drain event") }.unwrap());
    }
}
