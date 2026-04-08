//! I/O-free coroutine to send a timer request and receive a response.

use io_socket::{
    coroutines::{read::*, write::*},
    io::{SocketInput, SocketOutput},
};
use log::trace;
use thiserror::Error;

use crate::timer::{TimerRequest, TimerResponse};

/// Error emitted by the [`TimerRequestSend`] coroutine.
#[derive(Debug, Error)]
pub enum TimerRequestSendError {
    #[error("Failed to serialize timer request")]
    Serialize(#[source] serde_json::Error),
    #[error("Failed to deserialize timer response")]
    Deserialize(#[source] serde_json::Error),

    #[error("Reached EOF while receiving response")]
    ReadEof,
    #[error(transparent)]
    Read(SocketReadError),

    #[error("Reached unexpected EOF while sending request")]
    WriteEof,
    #[error(transparent)]
    Write(SocketWriteError),
}

/// Result emitted on each step of the [`TimerRequestSend`] coroutine.
#[derive(Debug)]
pub enum TimerRequestSendResult {
    /// The coroutine has successfully terminated its progression.
    Ok { response: TimerResponse },
    /// A socket I/O needs to be performed to make the coroutine
    /// progress.
    Io { input: SocketInput },
    /// The coroutine encountered an unrecoverable error.
    Err { err: TimerRequestSendError },
}

#[derive(Clone, Debug)]
enum State {
    Serialize,
    Write(SocketWrite),
    Read(SocketRead),
    Deserialize(Vec<u8>),
}

/// I/O-free coroutine to send a [`TimerRequest`] to a server and
/// receive the [`TimerResponse`] over NDJSON.
///
/// Each call to [`resume`] advances one step:
///
/// 1. Emit [`SocketInput::Write`] with the JSON-encoded request line.
/// 2. Emit [`SocketInput::Read`] to receive the JSON-encoded response
///    line.
/// 3. Return `Ok { response }`.
///
/// Use the named constructors ([`TimerRequestSend::get`],
/// [`TimerRequestSend::start`], …) rather than constructing directly.
///
/// [`resume`]: TimerRequestSend::resume
#[derive(Clone, Debug)]
pub struct TimerRequestSend {
    request: TimerRequest,
    state: State,
}

impl TimerRequestSend {
    pub fn new(request: TimerRequest) -> Self {
        trace!("timer request to send: {request:?}");

        Self {
            request,
            state: State::Serialize,
        }
    }

    /// Creates a coroutine that sends a [`TimerRequest::Get`].
    pub fn get() -> Self {
        Self::new(TimerRequest::Get)
    }

    /// Creates a coroutine that sends a [`TimerRequest::Start`].
    pub fn start() -> Self {
        Self::new(TimerRequest::Start)
    }

    /// Creates a coroutine that sends a [`TimerRequest::Stop`].
    pub fn stop() -> Self {
        Self::new(TimerRequest::Stop)
    }

    /// Creates a coroutine that sends a [`TimerRequest::Pause`].
    pub fn pause() -> Self {
        Self::new(TimerRequest::Pause)
    }

    /// Creates a coroutine that sends a [`TimerRequest::Resume`].
    pub fn resume_timer() -> Self {
        Self::new(TimerRequest::Resume)
    }

    /// Creates a coroutine that sends a [`TimerRequest::Update`].
    pub fn update() -> Self {
        Self::new(TimerRequest::Update)
    }

    /// Creates a coroutine that sends a [`TimerRequest::Set`].
    pub fn set(duration: usize) -> Self {
        Self::new(TimerRequest::Set(duration))
    }

    /// Advances the coroutine by one step.
    pub fn resume(&mut self, mut arg: Option<SocketOutput>) -> TimerRequestSendResult {
        loop {
            match &mut self.state {
                State::Serialize => match serde_json::to_vec(&self.request) {
                    Ok(mut bytes) => {
                        bytes.push(b'\n');
                        self.state = State::Write(SocketWrite::new(bytes));
                    }
                    Err(err) => {
                        let err = TimerRequestSendError::Serialize(err);
                        return TimerRequestSendResult::Err { err };
                    }
                },
                State::Write(w) => match w.resume(arg.take()) {
                    SocketWriteResult::Ok { .. } => {
                        self.state = State::Read(SocketRead::default());
                    }
                    SocketWriteResult::Io { input } => {
                        return TimerRequestSendResult::Io { input };
                    }
                    SocketWriteResult::Eof => {
                        let err = TimerRequestSendError::WriteEof;
                        return TimerRequestSendResult::Err { err };
                    }
                    SocketWriteResult::Err { err } => {
                        let err = TimerRequestSendError::Write(err);
                        return TimerRequestSendResult::Err { err };
                    }
                },
                State::Read(r) => match r.resume(arg.take()) {
                    SocketReadResult::Ok { mut buf, n } => {
                        buf.truncate(n);
                        self.state = State::Deserialize(buf);
                    }
                    SocketReadResult::Io { input } => {
                        return TimerRequestSendResult::Io { input };
                    }
                    SocketReadResult::Eof => {
                        let err = TimerRequestSendError::ReadEof;
                        return TimerRequestSendResult::Err { err };
                    }
                    SocketReadResult::Err { err } => {
                        let err = TimerRequestSendError::Read(err);
                        return TimerRequestSendResult::Err { err };
                    }
                },
                State::Deserialize(bytes) => {
                    let bytes = bytes.trim_ascii_end();
                    return match serde_json::from_slice(bytes) {
                        Ok(response) => {
                            trace!("timer response received: {response:?}");
                            TimerRequestSendResult::Ok { response }
                        }
                        Err(err) => TimerRequestSendResult::Err {
                            err: TimerRequestSendError::Deserialize(err),
                        },
                    };
                }
            }
        }
    }
}
