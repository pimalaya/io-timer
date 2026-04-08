//! Synchronous time runtime backed by [`std::time`].

use std::{
    io::{Error, ErrorKind, Result},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::io::{TimeInput, TimeOutput};

/// Processes a [`TimeInput`] request synchronously using
/// [`std::time`].
pub fn handle(input: TimeInput) -> Result<TimeOutput> {
    match input {
        TimeInput::Now => now(),
        TimeInput::Sleep { secs } => sleep(secs),
        TimeInput::SleepUntil { timestamp } => sleep_until(timestamp),
    }
}

/// Returns the current wall-clock time as a Unix timestamp.
pub fn now() -> Result<TimeOutput> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::new(ErrorKind::Other, err))?;

    let secs = now.as_secs();
    let nanos = now.subsec_nanos();

    Ok(TimeOutput::Now { secs, nanos })
}

/// Blocks for the given number of seconds.
pub fn sleep(secs: u64) -> Result<TimeOutput> {
    thread::sleep(Duration::from_secs(secs));
    Ok(TimeOutput::Slept)
}

/// Blocks until the given Unix epoch second is reached.
pub fn sleep_until(timestamp: u64) -> Result<TimeOutput> {
    let target = Duration::from_secs(timestamp);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::new(ErrorKind::Other, err))?;

    if target > now {
        thread::sleep(target - now);
    }

    Ok(TimeOutput::Slept)
}
