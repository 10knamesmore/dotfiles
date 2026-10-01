//! Error categories used at the Python boundary.

/// An error that is either caused by invalid caller input or by a runtime failure.
#[derive(Debug)]
pub(crate) enum TerminalError {
    InvalidArgument(String),
    Runtime(String),
}

impl TerminalError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    pub(crate) fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

pub(crate) type TerminalResult<T> = Result<T, TerminalError>;
