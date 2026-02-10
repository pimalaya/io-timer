use io_stream::{
    coroutines::{
        read::{ReadStream, ReadStreamError, ReadStreamResult},
        write::{WriteStream, WriteStreamError, WriteStreamResult},
    },
    io::StreamIo,
};
use log::debug;
use memchr::memrchr;
use thiserror::Error;

use crate::{Request, Response};

#[derive(Debug)]
pub enum State {
    SendRequest(WriteStream),
    ReceiveResponse(ReadStream),
}

/// Output emitted after a coroutine finishes its progression.
#[derive(Clone, Debug)]
pub enum SendRequestResult {
    /// The coroutine has successfully terminated its progression.
    Ok(Response),

    /// A stream I/O needs to be performed to make the coroutine
    /// progress.
    Io(StreamIo),

    /// An error occured during the coroutine progression.
    Err(SendRequestError),
}

#[derive(Clone, Debug, Error)]
pub enum SendRequestError {
    #[error("Received unexpected EOF")]
    Eof,

    #[error(transparent)]
    ReadStream(#[from] ReadStreamError),
    #[error(transparent)]
    WriteStream(#[from] WriteStreamError),
}

#[derive(Debug)]
pub struct SendRequest {
    state: State,
    response: Vec<u8>,
}

impl SendRequest {
    pub fn new(request: Request) -> Self {
        let coroutine = WriteStream::new(request.to_vec());
        let state = State::SendRequest(coroutine);
        let response = Vec::new();

        Self { state, response }
    }

    pub fn resume(&mut self, mut arg: Option<StreamIo>) -> SendRequestResult {
        loop {
            match &mut self.state {
                State::SendRequest(write) => {
                    match write.resume(arg.take()) {
                        WriteStreamResult::Ok(_) => (),
                        WriteStreamResult::Eof => {
                            return SendRequestResult::Err(SendRequestError::Eof)
                        }
                        WriteStreamResult::Err(err) => {
                            return SendRequestResult::Err(err.into());
                        }
                        WriteStreamResult::Io(io) => {
                            debug!("need to send response");
                            return SendRequestResult::Io(io);
                        }
                    };

                    let read = ReadStream::default();
                    self.state = State::ReceiveResponse(read);
                }
                State::ReceiveResponse(read) => {
                    let output = match read.resume(arg.take()) {
                        ReadStreamResult::Ok(output) => output,
                        ReadStreamResult::Eof => {
                            return SendRequestResult::Err(SendRequestError::Eof)
                        }
                        ReadStreamResult::Err(err) => {
                            return SendRequestResult::Err(err.into());
                        }
                        ReadStreamResult::Io(io) => {
                            debug!("need to receive request chunk");
                            return SendRequestResult::Io(io);
                        }
                    };

                    let bytes = output.bytes();

                    match memrchr(b'\n', bytes) {
                        Some(n) => {
                            self.response.extend(&bytes[..n]);
                            let response = serde_json::from_slice(&self.response).unwrap();
                            debug!("got complete response: {response:?}");
                            break SendRequestResult::Ok(response);
                        }
                        None => {
                            debug!("no new line found, need more response chunks");
                            self.response.extend(bytes);
                            read.replace(output.buffer);
                            continue;
                        }
                    }
                }
            }
        }
    }
}
