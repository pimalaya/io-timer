//! Time I/O request and response types.

/// I/O request emitted by time coroutines.
#[derive(Clone, Debug)]
pub enum TimeInput {
    /// Request the current time.
    Now,
    /// Sleep for the given number of seconds, then continue.
    Sleep { secs: u64 },
    /// Sleep until the given Unix epoch second is reached, then
    /// continue.
    SleepUntil { timestamp: u64 },
}

/// I/O response returned by time runtimes.
#[derive(Clone, Debug)]
pub enum TimeOutput {
    /// The current time as a Unix timestamp.
    Now {
        /// Whole seconds since the Unix epoch.
        secs: u64,
        /// Sub-second nanoseconds component.
        nanos: u32,
    },
    /// The requested sleep has completed.
    Slept,
}
