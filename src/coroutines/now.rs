//! I/O-free coroutine to get the current time.

use log::{debug, trace};
use thiserror::Error;

use crate::io::{TimeInput, TimeOutput};

/// Error emitted by the [`TimeNow`] coroutine.
#[derive(Clone, Debug, Error)]
pub enum TimeNowError {
    #[error("Invalid time now arg: {0:?}")]
    InvalidArg(TimeOutput),
}

/// Result emitted on each step of the [`TimeNow`] coroutine.
#[derive(Clone, Debug)]
pub enum TimeNowResult {
    /// The coroutine has successfully terminated its progression.
    Ok { secs: u64, nanos: u32 },
    /// A time I/O needs to be performed to make the coroutine
    /// progress.
    Io { input: TimeInput },
    /// The coroutine encountered an unrecoverable error.
    Err { err: TimeNowError },
}

/// I/O-free coroutine to get the current time as a Unix timestamp.
///
/// Emits a single [`TimeInput::Now`] request and returns `(secs,
/// nanos)` once the runtime responds.
#[derive(Clone, Debug, Default)]
pub struct TimeNow;

impl TimeNow {
    /// Creates a new coroutine.
    pub fn new() -> Self {
        Self
    }

    /// Makes the progress.
    pub fn resume(&mut self, arg: Option<TimeOutput>) -> TimeNowResult {
        match arg {
            None => {
                trace!("wants I/O to get current time");
                TimeNowResult::Io {
                    input: TimeInput::Now,
                }
            }
            Some(TimeOutput::Now { secs, nanos }) => {
                debug!("resume after getting current time: {secs}s {nanos}ns");
                TimeNowResult::Ok { secs, nanos }
            }
            Some(output) => {
                let err = TimeNowError::InvalidArg(output);
                TimeNowResult::Err { err }
            }
        }
    }
}
