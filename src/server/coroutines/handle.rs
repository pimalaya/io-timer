use std::mem;

use io_stream::{
    coroutines::{
        read::{ReadStream, ReadStreamError, ReadStreamResult},
        write::{WriteStream, WriteStreamError, WriteStreamResult},
    },
    io::StreamIo,
};
use log::{debug, trace};
use memchr::memrchr;
use thiserror::Error;

use crate::{timer::TimerEvent, Request, Response, Timer};

#[derive(Debug)]
pub enum State {
    ReceiveRequest(ReadStream),
    SendResponse(WriteStream),
}

/// Output emitted after a coroutine finishes its progression.
#[derive(Clone, Debug)]
pub enum HandleRequestResult {
    /// The coroutine has successfully terminated its progression.
    Ok(Vec<TimerEvent>),

    /// A stream I/O needs to be performed to make the coroutine
    /// progress.
    Io(StreamIo),

    /// An error occured during the coroutine progression.
    Err(HandleRequestError),
}

#[derive(Clone, Debug, Error)]
pub enum HandleRequestError {
    #[error("Received unexpected EOF")]
    Eof,

    #[error(transparent)]
    ReadStream(#[from] ReadStreamError),
    #[error(transparent)]
    WriteStream(#[from] WriteStreamError),
}

#[derive(Debug)]
pub struct HandleRequest {
    state: State,
    request: Vec<u8>,
    events: Vec<TimerEvent>,
}

impl HandleRequest {
    pub fn new() -> Self {
        Self {
            state: State::ReceiveRequest(ReadStream::default()),
            request: Vec::new(),
            events: Vec::with_capacity(2),
        }
    }

    pub fn resume(
        &mut self,
        timer: &mut Timer,
        mut input: Option<StreamIo>,
    ) -> HandleRequestResult {
        loop {
            match &mut self.state {
                State::ReceiveRequest(read) => {
                    let output = match read.resume(input.take()) {
                        ReadStreamResult::Ok(output) => output,
                        ReadStreamResult::Eof => {
                            return HandleRequestResult::Err(HandleRequestError::Eof)
                        }
                        ReadStreamResult::Err(err) => {
                            return HandleRequestResult::Err(err.into());
                        }
                        ReadStreamResult::Io(io) => {
                            debug!("need to receive request chunk");
                            return HandleRequestResult::Io(io);
                        }
                    };

                    let bytes = output.bytes();
                    let request = match memrchr(b'\n', bytes) {
                        Some(n) => {
                            self.request.extend(&bytes[..n]);
                            let request = serde_json::from_slice(&self.request).unwrap();
                            debug!("got complete request: {request:?}");
                            request
                        }
                        None => {
                            debug!("no new line found, need more request chunks");
                            self.request.extend(bytes);
                            read.replace(output.buffer);
                            continue;
                        }
                    };

                    let response = match request {
                        Request::Start => {
                            self.events.extend(timer.start());
                            Response::Ok
                        }
                        Request::Get => Response::Timer(timer.clone()),
                        Request::Set(duration) => {
                            self.events.extend(timer.set(duration));
                            Response::Ok
                        }
                        Request::Pause => {
                            timer.pause();
                            Response::Ok
                        }
                        Request::Resume => {
                            self.events.extend(timer.resume());
                            Response::Ok
                        }
                        Request::Stop => {
                            self.events.extend(timer.stop());
                            Response::Ok
                        }
                    };

                    debug!("successfully process request: {response:?}");

                    let coroutine = WriteStream::new(response.to_vec());
                    self.state = State::SendResponse(coroutine);
                }
                State::SendResponse(write) => {
                    match write.resume(input.take()) {
                        WriteStreamResult::Ok(_) => (),
                        WriteStreamResult::Eof => {
                            return HandleRequestResult::Err(HandleRequestError::Eof)
                        }
                        WriteStreamResult::Err(err) => {
                            return HandleRequestResult::Err(err.into());
                        }
                        WriteStreamResult::Io(io) => {
                            debug!("need to send response");
                            return HandleRequestResult::Io(io);
                        }
                    };

                    let events = mem::take(&mut self.events);
                    debug!("generated {} events to be processed", events.len());
                    trace!("{events:#?}");
                    break HandleRequestResult::Ok(events);
                }
            }
        }
    }
}
