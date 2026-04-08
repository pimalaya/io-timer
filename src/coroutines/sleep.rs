//! I/O-free coroutine to sleep for a given number of seconds.

use log::{debug, trace};
use thiserror::Error;

use crate::io::{TimeInput, TimeOutput};

/// Error emitted by the [`TimeSleep`] coroutine.
#[derive(Clone, Debug, Error)]
pub enum TimeSleepError {
    #[error("Invalid time sleep arg: {0:?}")]
    InvalidArg(TimeOutput),
}

/// Result emitted on each step of the [`TimeSleep`] coroutine.
#[derive(Clone, Debug)]
pub enum TimeSleepResult {
    /// The coroutine has successfully terminated its progression.
    Ok,
    /// A time I/O needs to be performed to make the coroutine
    /// progress.
    Io { input: TimeInput },
    /// The coroutine encountered an unrecoverable error.
    Err { err: TimeSleepError },
}

/// I/O-free coroutine to sleep for a given number of seconds.
///
/// Emits a single [`TimeInput::Sleep`] request and returns `Ok` once
/// the runtime signals the sleep has completed.
///
/// Use [`TimeSleepUntil`] to sleep until a specific Unix timestamp.
///
/// [`TimeSleepUntil`]: crate::coroutines::sleep_until::TimeSleepUntil
#[derive(Clone, Debug)]
pub struct TimeSleep {
    secs: u64,
}

impl TimeSleep {
    /// Creates a new coroutine that sleeps for `secs` seconds.
    pub fn new(secs: u64) -> Self {
        Self { secs }
    }

    /// Makes the sleep progress.
    pub fn resume(&mut self, arg: Option<TimeOutput>) -> TimeSleepResult {
        match arg {
            None => {
                let secs = self.secs;
                trace!("wants time I/O to sleep for {secs}s");
                TimeSleepResult::Io {
                    input: TimeInput::Sleep { secs },
                }
            }
            Some(TimeOutput::Slept) => {
                debug!("resume after sleeping");
                TimeSleepResult::Ok
            }
            Some(output) => {
                let err = TimeSleepError::InvalidArg(output);
                TimeSleepResult::Err { err }
            }
        }
    }
}
