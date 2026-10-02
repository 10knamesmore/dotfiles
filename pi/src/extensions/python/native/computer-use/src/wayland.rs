//! Own Wayland protocol objects and dispatch events through interruptible polls.

use std::collections::HashMap;
use std::os::fd::{AsFd, AsRawFd};
use std::time::{Duration, Instant};

use pyo3::prelude::*;
use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_output, wl_registry, wl_seat, wl_shm, wl_shm_pool,
};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, WEnum, delegate_noop};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1, zwlr_screencopy_manager_v1,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

use crate::{logging, wait};

pub(crate) use zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1 as VirtualPointer;
pub(crate) use zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1 as VirtualKeyboard;

/// One compositor-advertised output, removed when its registry global disappears.
pub(crate) struct Output {
    /// Bound wl_output v4 used for capture and output-local pointer mapping.
    pub proxy: wl_output::WlOutput,

    /// Connector name supplied by wl_output.name.
    pub name: String,

    /// Wayland rotation/reflection value in 0..=7 for the raw capture buffer.
    pub transform: u32,
}

/// Accumulate metadata and completion events for the connection's single capture.
#[derive(Default)]
pub(crate) struct Frame {
    /// Shared-memory format offered by the compositor; absent if unsupported.
    pub format: Option<wl_shm::Format>,

    /// Raw buffer width in pixels before orientation correction.
    pub width: u32,

    /// Raw buffer height in pixels before orientation correction.
    pub height: u32,

    /// Byte distance between adjacent rows, including any padding.
    pub stride: u32,

    /// The compositor has finished advertising buffer formats.
    pub buffer_done: bool,

    /// Raw rows require vertical reversal before applying output rotation.
    pub y_invert: bool,

    /// The compositor has finished writing the supplied buffer.
    pub ready: bool,

    /// The compositor rejected or failed this capture.
    pub failed: bool,
}

/// Registry bindings and event results owned by one connection's queue.
#[derive(Default)]
pub(crate) struct State {
    /// Outputs indexed by registry global id, updated on hotplug events.
    pub outputs: HashMap<u32, Output>,

    /// Allocates shared-memory buffers for screenshots.
    pub shm: Option<wl_shm::WlShm>,

    /// Seat to which this worker's virtual devices attach.
    pub seat: Option<wl_seat::WlSeat>,

    /// Capture manager; binding it does not request a screenshot.
    pub screencopy: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,

    /// Pointer factory; devices are created only by input operations.
    pub pointer_manager: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,

    /// Keyboard factory; devices are created only by input operations.
    pub keyboard_manager: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,

    /// Capture results used only by screenshot connections.
    pub frame: Frame,

    /// Last acknowledged sync token; cancelled waits can leave earlier callbacks queued.
    sync_completed: u64,
}

/// Keep one independent connection for captures or for worker-owned input devices.
pub(crate) struct Desktop {
    /// Socket and protocol object ownership; dropping it disconnects these devices.
    pub connection: Connection,

    /// Dispatches this connection's events on the calling Python thread.
    pub queue: EventQueue<State>,

    /// Registry and request results populated by dispatch.
    pub state: State,

    /// Monotonic request token preventing stale callbacks from completing cleanup.
    next_sync: u64,
}

impl Desktop {
    pub(crate) fn connect(py: Python<'_>) -> PyResult<Self> {
        let display = std::env::var_os("WAYLAND_DISPLAY")
            .ok_or_else(|| wait::runtime("WAYLAND_DISPLAY is not set"))?;
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .ok_or_else(|| wait::runtime("XDG_RUNTIME_DIR is not set"))?;
        let socket = crate::hyprland::socket_connect(
            py,
            &std::path::PathBuf::from(runtime).join(display),
            Instant::now() + Duration::from_secs(5),
        )?;
        let connection = Connection::from_socket(socket).map_err(wait::runtime)?;
        let queue = connection.new_event_queue();
        connection.display().get_registry(&queue.handle(), ());
        let mut desktop = Self {
            connection,
            queue,
            state: State::default(),
            next_sync: 0,
        };
        desktop.sync(py, true)?;
        desktop.sync(py, true)?;
        logging::event("wayland.connect", "ok");
        Ok(desktop)
    }

    pub(crate) fn sync(&mut self, py: Python<'_>, signals: bool) -> PyResult<()> {
        self.next_sync += 1;
        let token = self.next_sync;
        self.connection.display().sync(&self.queue.handle(), token);
        self.wait_for(py, signals, |state| state.sync_completed >= token)
    }

    pub(crate) fn wait_for(
        &mut self,
        py: Python<'_>,
        signals: bool,
        ready: impl Fn(&State) -> bool,
    ) -> PyResult<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if Instant::now() >= deadline {
                return Err(pyo3::exceptions::PyTimeoutError::new_err(
                    "Wayland request timed out",
                ));
            }
            if signals {
                py.check_signals()?;
            }
            self.queue
                .dispatch_pending(&mut self.state)
                .map_err(wait::runtime)?;
            if ready(&self.state) {
                return Ok(());
            }
            let writable = match self.connection.flush() {
                Ok(()) => false,
                Err(wayland_client::backend::WaylandError::Io(error))
                    if error.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    true
                }
                Err(error) => return Err(wait::runtime(error)),
            };
            let Some(guard) = self.queue.prepare_read() else {
                continue;
            };
            let events = libc::POLLIN | if writable { libc::POLLOUT } else { 0 };
            if wait::poll(
                py,
                self.connection.as_fd().as_raw_fd(),
                events,
                deadline,
                signals,
            )? {
                match guard.read() {
                    Ok(_) => {}
                    Err(wayland_client::backend::WaylandError::Io(error))
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) => return Err(wait::runtime(error)),
                }
            }
        }
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        logging::event("wayland.close", "ok");
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "wl_output" if version >= 4 => {
                    let proxy = registry.bind(name, 4, qh, name);
                    state.outputs.insert(
                        name,
                        Output {
                            proxy,
                            name: String::new(),
                            transform: 0,
                        },
                    );
                }
                "wl_shm" => state.shm = Some(registry.bind(name, 1, qh, ())),
                "wl_seat" if state.seat.is_none() => {
                    state.seat = Some(registry.bind(name, 1, qh, ()))
                }
                "zwlr_screencopy_manager_v1" if version >= 3 => {
                    state.screencopy = Some(registry.bind(name, 3, qh, ()))
                }
                "zwlr_virtual_pointer_manager_v1" if version >= 2 => {
                    state.pointer_manager = Some(registry.bind(name, 2, qh, ()))
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    state.keyboard_manager = Some(registry.bind(name, 1, qh, ()))
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(output) = state.outputs.remove(&name) {
                    output.proxy.release();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for State {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        id: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Some(output) = state.outputs.get_mut(id) {
            match event {
                wl_output::Event::Name { name } => output.name = name,
                wl_output::Event::Geometry {
                    transform: WEnum::Value(transform),
                    ..
                } => output.transform = transform as u32,
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_callback::WlCallback, u64> for State {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        token: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.sync_completed = *token;
    }
}

impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use zwlr_screencopy_frame_v1::Event;
        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                state.frame.format = format.into_result().ok();
                state.frame.width = width;
                state.frame.height = height;
                state.frame.stride = stride;
            }
            Event::BufferDone => state.frame.buffer_done = true,
            Event::Flags { flags } => state.frame.y_invert = u32::from(flags) & 1 != 0,
            Event::Ready { .. } => state.frame.ready = true,
            Event::Failed => state.frame.failed = true,
            _ => {}
        }
    }
}

delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);
delegate_noop!(State: ignore wl_seat::WlSeat);
delegate_noop!(State: ignore zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1);
delegate_noop!(State: ignore zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1);
delegate_noop!(State: ignore VirtualPointer);
delegate_noop!(State: ignore zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1);
delegate_noop!(State: ignore VirtualKeyboard);
