//! I/O-free coroutine to sleep until a given Unix timestamp.

use log::{debug, trace};
use thiserror::Error;

use crate::io::{TimeInput, TimeOutput};

/// Error emitted by the [`TimeSleepUntil`] coroutine.
#[derive(Clone, Debug, Error)]
pub enum TimeSleepUntilError {
    #[error("Invalid time sleep until arg: {0:?}")]
    InvalidArg(TimeOutput),
}

/// Result emitted on each step of the [`TimeSleepUntil`] coroutine.
#[derive(Clone, Debug)]
pub enum TimeSleepUntilResult {
    /// The coroutine has successfully terminated its progression.
    Ok,
    /// A time I/O needs to be performed to make the coroutine
    /// progress.
    Io { input: TimeInput },
    /// The coroutine encountered an unrecoverable error.
    Err { err: TimeSleepUntilError },
}

/// I/O-free coroutine to sleep until a given Unix epoch second.
///
/// Emits a single [`TimeInput::SleepUntil`] request and returns `Ok`
/// once the runtime signals the sleep has completed.
///
/// Use [`TimeSleep`] to sleep for a relative duration instead.
///
/// [`TimeSleep`]: crate::coroutines::sleep::TimeSleep
#[derive(Clone, Debug)]
pub struct TimeSleepUntil {
    timestamp: u64,
}

impl TimeSleepUntil {
    /// Creates a new coroutine that sleeps until `timestamp` (Unix
    /// epoch seconds).
    pub fn new(timestamp: u64) -> Self {
        Self { timestamp }
    }

    /// Makes the sleep progress.
    pub fn resume(&mut self, arg: Option<TimeOutput>) -> TimeSleepUntilResult {
        match arg {
            None => {
                let timestamp = self.timestamp;
                trace!("wants time I/O to sleep until {timestamp}s");
                TimeSleepUntilResult::Io {
                    input: TimeInput::SleepUntil { timestamp },
                }
            }
            Some(TimeOutput::Slept) => {
                debug!("resume after sleeping until {}", self.timestamp);
                TimeSleepUntilResult::Ok
            }
            Some(output) => {
                let err = TimeSleepUntilError::InvalidArg(output);
                TimeSleepUntilResult::Err { err }
            }
        }
    }
}
