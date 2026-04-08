//! I/O-free coroutine to receive a timer request and send a response.

use alloc::vec::Vec;

use io_socket::{
    coroutines::{read::*, write::*},
    io::{SocketInput, SocketOutput},
};
use log::{debug, trace};
use thiserror::Error;

use crate::{
    coroutines::now::{TimeNow, TimeNowError, TimeNowResult},
    io::{TimeInput, TimeOutput},
    timer::{Timer, TimerEvent, TimerRequest, TimerResponse},
};

/// Unified argument passed back to [`TimerRequestHandle`] after any
/// I/O, since it mixes both time and socket I/O.
#[derive(Clone, Debug)]
pub enum TimerRequestHandleArg {
    /// Response to a [`TimeInput`] request.
    Time(TimeOutput),
    /// Response to a [`SocketInput`] request.
    Socket(SocketOutput),
}

impl From<TimeOutput> for TimerRequestHandleArg {
    fn from(output: TimeOutput) -> Self {
        Self::Time(output)
    }
}

impl From<SocketOutput> for TimerRequestHandleArg {
    fn from(output: SocketOutput) -> Self {
        Self::Socket(output)
    }
}

/// Error emitted by the [`TimerRequestHandle`] coroutine.
#[derive(Debug, Error)]
pub enum TimerRequestHandleError {
    #[error("Invalid timer request handle arg: {0:?}")]
    InvalidArg(TimerRequestHandleArg),

    #[error(transparent)]
    TimeNow(TimeNowError),

    #[error("Failed to serialize timer response")]
    Serialize(#[source] serde_json::Error),
    #[error("Failed to deserialize timer request")]
    Deserialize(#[source] serde_json::Error),

    #[error("Reached unexpected EOF while reading request")]
    ReadEof,
    #[error(transparent)]
    Read(SocketReadError),

    #[error("Reached unexpected EOF while writing response")]
    WriteEof,
    #[error(transparent)]
    Write(SocketWriteError),
}

/// Result emitted on each step of the [`TimerRequestHandle`]
/// coroutine.
#[derive(Debug)]
pub enum TimerRequestHandleResult {
    /// The coroutine has successfully terminated its progression.
    Ok { events: Vec<TimerEvent> },
    /// A socket I/O needs to be performed to make the coroutine
    /// progress.
    Io { input: SocketInput },
    /// A time I/O needs to be performed to make the coroutine
    /// progress.
    TimeIo { input: TimeInput },
    /// The coroutine encountered an unrecoverable error.
    Err { err: TimerRequestHandleError },
}

#[derive(Clone, Debug)]
enum State {
    Read(SocketRead),
    Deserialize(Vec<u8>),
    GetTime(Option<TimerRequest>, TimeNow),
    Write(SocketWrite),
}

/// I/O-free coroutine to handle one complete timer request-response
/// cycle over NDJSON.
///
/// Each call to [`resume`] advances one step:
///
/// 1. Emit [`SocketInput::Read`] to receive a JSON-encoded
///    [`TimerRequest`] line.
/// 2. Optionally emit [`TimeInput::Now`] for time-dependent requests,
///    driven by a [`TimeNow`] sub-coroutine.
/// 3. Apply the request to the [`Timer`].
/// 4. Emit [`SocketInput::Write`] with the JSON-encoded
///    [`TimerResponse`] line.
/// 5. Return `Ok { events }` once the write completes.
///
/// [`resume`]: TimerRequestHandle::resume
#[derive(Debug)]
pub struct TimerRequestHandle {
    state: State,
    events: Option<Vec<TimerEvent>>,
}

impl TimerRequestHandle {
    /// Creates a new coroutine.
    pub fn new() -> Self {
        Self {
            state: State::Read(SocketRead::default()),
            events: None,
        }
    }

    /// Advances the coroutine by one step.
    pub fn resume(
        &mut self,
        timer: &mut Timer,
        mut arg: Option<impl Into<TimerRequestHandleArg>>,
    ) -> TimerRequestHandleResult {
        loop {
            match &mut self.state {
                State::Read(r) => {
                    let socket_arg = match arg.take().map(Into::into) {
                        None => None,
                        Some(TimerRequestHandleArg::Socket(output)) => Some(output),
                        Some(a) => {
                            let err = TimerRequestHandleError::InvalidArg(a);
                            return TimerRequestHandleResult::Err { err };
                        }
                    };
                    match r.resume(socket_arg) {
                        SocketReadResult::Ok { mut buf, n } => {
                            buf.truncate(n);
                            self.state = State::Deserialize(buf);
                        }
                        SocketReadResult::Io { input } => {
                            return TimerRequestHandleResult::Io { input };
                        }
                        SocketReadResult::Eof => {
                            let err = TimerRequestHandleError::ReadEof;
                            return TimerRequestHandleResult::Err { err };
                        }
                        SocketReadResult::Err { err } => {
                            let err = TimerRequestHandleError::Read(err);
                            return TimerRequestHandleResult::Err { err };
                        }
                    }
                }
                State::Deserialize(bytes) => {
                    let bytes = bytes.trim_ascii_end();
                    let request: TimerRequest = match serde_json::from_slice(bytes) {
                        Ok(r) => r,
                        Err(e) => {
                            let err = TimerRequestHandleError::Deserialize(e);
                            return TimerRequestHandleResult::Err { err };
                        }
                    };
                    debug!("received request: {request:?}");
                    match request {
                        TimerRequest::Get | TimerRequest::Stop | TimerRequest::Set(_) => {
                            match self.serialize_response(timer, &request, None) {
                                Ok(bytes) => self.state = State::Write(SocketWrite::new(bytes)),
                                Err(err) => return TimerRequestHandleResult::Err { err },
                            }
                        }
                        _ => {
                            trace!("wants time I/O before processing request");
                            self.state = State::GetTime(Some(request), TimeNow::new());
                        }
                    }
                }
                State::GetTime(request, time_now) => {
                    let time_arg = match arg.take().map(Into::into) {
                        None => None,
                        Some(TimerRequestHandleArg::Time(output)) => Some(output),
                        Some(a) => {
                            let err = TimerRequestHandleError::InvalidArg(a);
                            return TimerRequestHandleResult::Err { err };
                        }
                    };
                    match time_now.resume(time_arg) {
                        TimeNowResult::Ok { secs, .. } => {
                            let request = request.take().unwrap();
                            match self.serialize_response(timer, &request, Some(secs)) {
                                Ok(bytes) => self.state = State::Write(SocketWrite::new(bytes)),
                                Err(err) => return TimerRequestHandleResult::Err { err },
                            }
                        }
                        TimeNowResult::Io { input } => {
                            return TimerRequestHandleResult::TimeIo { input };
                        }
                        TimeNowResult::Err { err } => {
                            let err = TimerRequestHandleError::TimeNow(err);
                            return TimerRequestHandleResult::Err { err };
                        }
                    }
                }
                State::Write(w) => {
                    let socket_arg = match arg.take().map(Into::into) {
                        None => None,
                        Some(TimerRequestHandleArg::Socket(output)) => Some(output),
                        Some(a) => {
                            let err = TimerRequestHandleError::InvalidArg(a);
                            return TimerRequestHandleResult::Err { err };
                        }
                    };

                    return match w.resume(socket_arg) {
                        SocketWriteResult::Ok { .. } => {
                            let events = self.events.take().unwrap_or_default();
                            TimerRequestHandleResult::Ok { events }
                        }
                        SocketWriteResult::Io { input } => TimerRequestHandleResult::Io { input },
                        SocketWriteResult::Eof => {
                            let err = TimerRequestHandleError::WriteEof;
                            TimerRequestHandleResult::Err { err }
                        }
                        SocketWriteResult::Err { err } => {
                            let err = TimerRequestHandleError::Write(err);
                            TimerRequestHandleResult::Err { err }
                        }
                    };
                }
            }
        }
    }

    fn serialize_response(
        &mut self,
        timer: &mut Timer,
        request: &TimerRequest,
        secs: Option<u64>,
    ) -> Result<Vec<u8>, TimerRequestHandleError> {
        let response = match request {
            TimerRequest::Get => TimerResponse::Timer(timer.clone()),
            TimerRequest::Stop => TimerResponse::Events(timer.stop().into_iter().collect()),
            TimerRequest::Set(d) => TimerResponse::Events(timer.set(*d).into_iter().collect()),
            TimerRequest::Start => {
                TimerResponse::Events(timer.start(secs.unwrap()).into_iter().collect())
            }
            TimerRequest::Pause => {
                TimerResponse::Events(timer.pause(secs.unwrap()).into_iter().collect())
            }
            TimerRequest::Resume => {
                TimerResponse::Events(timer.resume(secs.unwrap()).into_iter().collect())
            }
            TimerRequest::Update => {
                TimerResponse::Events(timer.update(secs.unwrap()).into_iter().collect())
            }
        };

        self.events = Some(match &response {
            TimerResponse::Events(events) => events.clone(),
            TimerResponse::Timer(_) => Vec::new(),
        });

        let mut bytes =
            serde_json::to_vec(&response).map_err(TimerRequestHandleError::Serialize)?;
        bytes.push(b'\n');

        Ok(bytes)
    }
}
