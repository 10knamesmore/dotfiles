//! Encode semantic input against the terminal owned by the emulator thread.

use libghostty_vt::{Terminal, mouse, terminal::Mode};

use crate::error::{TerminalError, TerminalResult};
use crate::model::{
    CursorKey, InputSpec, KeySpec, MouseAction, MouseInput, MouseModes, PixelSize, Size,
};

/// Ghostty handles stay on the emulator thread; no mouse position or held button is retained.
pub(super) struct InputEncoder {
    mouse: mouse::Encoder<'static>,
    event: mouse::Event<'static>,
}

impl InputEncoder {
    pub(super) fn new() -> Result<Self, String> {
        Ok(Self {
            mouse: mouse::Encoder::new()
                .map_err(|error| format!("create mouse encoder failed: {error}"))?,
            event: mouse::Event::new()
                .map_err(|error| format!("create mouse event failed: {error}"))?,
        })
    }

    /// Encode the whole batch before the caller writes any bytes to the PTY.
    pub(super) fn encode(
        &mut self,
        terminal: &Terminal<'_, '_>,
        cell_size: PixelSize,
        inputs: &[InputSpec],
    ) -> TerminalResult<Vec<u8>> {
        let size = Size {
            cols: terminal.cols().map_err(encoding_error)?,
            rows: terminal.rows().map_err(encoding_error)?,
        };
        for input in inputs {
            if let InputSpec::Mouse(mouse) = input {
                mouse.validate_position(size)?;
            }
        }
        let application_cursor = terminal.mode(Mode::DECCKM).map_err(encoding_error)?;
        self.mouse
            .set_options_from_terminal(terminal)
            .set_size(mouse::EncoderSize {
                screen_width: u32::from(size.cols) * u32::from(cell_size.width),
                screen_height: u32::from(size.rows) * u32::from(cell_size.height),
                cell_width: u32::from(cell_size.width),
                cell_height: u32::from(cell_size.height),
                padding_top: 0,
                padding_bottom: 0,
                padding_left: 0,
                padding_right: 0,
            });

        let mut output = Vec::new();
        for input in inputs {
            match input {
                InputSpec::Bytes(bytes) | InputSpec::Key(KeySpec::Bytes(bytes)) => {
                    output.extend_from_slice(bytes);
                }
                InputSpec::Key(KeySpec::Cursor(key)) => {
                    output.extend_from_slice(encode_cursor(*key, application_cursor));
                }
                InputSpec::Mouse(mouse) => self.encode_mouse(mouse, cell_size, &mut output)?,
            }
        }
        Ok(output)
    }

    fn encode_mouse(
        &mut self,
        input: &MouseInput,
        cell_size: PixelSize,
        output: &mut Vec<u8>,
    ) -> TerminalResult<()> {
        let (action, button, repetitions) = match input.action {
            MouseAction::Press(button) => (mouse::Action::Press, Some(button), 1),
            MouseAction::Release(button) => (mouse::Action::Release, Some(button), 1),
            MouseAction::Move(button) => (mouse::Action::Motion, button, 1),
            MouseAction::Scroll { button, steps } => (mouse::Action::Press, Some(button), steps),
        };
        self.mouse
            .set_any_button_pressed(button.is_some() && action != mouse::Action::Release);
        self.event
            .set_action(action)
            .set_button(button)
            .set_mods(input.modifiers)
            .set_position(mouse::Position {
                x: (f32::from(input.col) + 0.5) * f32::from(cell_size.width),
                y: (f32::from(input.row) + 0.5) * f32::from(cell_size.height),
            });
        // Coordinates fit the PTY's u16 pixel geometry; 32 bytes hold every supported format.
        let mut encoded = [0_u8; 32];
        let length = self
            .mouse
            .encode(&self.event, &mut encoded)
            .map_err(encoding_error)?;
        for _ in 0..repetitions {
            output.extend_from_slice(&encoded[..length]);
        }
        Ok(())
    }
}

/// Read requested mode bits without imposing a precedence on Ghostty's effective mouse state.
pub(super) fn mouse_modes(terminal: &Terminal<'_, '_>) -> Result<MouseModes, String> {
    let mode = |mode| {
        terminal
            .mode(mode)
            .map_err(|error| format!("read mouse mode failed: {error}"))
    };
    Ok(MouseModes {
        x10: mode(Mode::X10_MOUSE)?,
        normal: mode(Mode::NORMAL_MOUSE)?,
        button: mode(Mode::BUTTON_MOUSE)?,
        any: mode(Mode::ANY_MOUSE)?,
        utf8: mode(Mode::UTF8_MOUSE)?,
        sgr: mode(Mode::SGR_MOUSE)?,
        urxvt: mode(Mode::URXVT_MOUSE)?,
        sgr_pixels: mode(Mode::SGR_PIXELS_MOUSE)?,
    })
}

fn encode_cursor(key: CursorKey, application_cursor: bool) -> &'static [u8] {
    match (key, application_cursor) {
        (CursorKey::Up, false) => b"\x1b[A",
        (CursorKey::Down, false) => b"\x1b[B",
        (CursorKey::Left, false) => b"\x1b[D",
        (CursorKey::Right, false) => b"\x1b[C",
        (CursorKey::Home, false) => b"\x1b[H",
        (CursorKey::End, false) => b"\x1b[F",
        (CursorKey::Up, true) => b"\x1bOA",
        (CursorKey::Down, true) => b"\x1bOB",
        (CursorKey::Left, true) => b"\x1bOD",
        (CursorKey::Right, true) => b"\x1bOC",
        (CursorKey::Home, true) => b"\x1bOH",
        (CursorKey::End, true) => b"\x1bOF",
    }
}

fn encoding_error(error: libghostty_vt::error::Error) -> TerminalError {
    TerminalError::runtime(format!("encode terminal input failed: {error}"))
}
