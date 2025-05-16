use io_stream::{
    coroutines::{Read, Write},
    Io,
};
use log::debug;
use memchr::memrchr;

use crate::{Request, Response};

#[derive(Debug)]
pub enum State {
    SendRequest(Write),
    ReceiveResponse(Read),
}

#[derive(Debug)]
pub struct SendRequest {
    state: State,
    response: Vec<u8>,
}

impl SendRequest {
    pub fn new(request: Request) -> Self {
        let coroutine = Write::new(request.to_vec());
        let state = State::SendRequest(coroutine);
        let response = Vec::new();

        Self { state, response }
    }

    pub fn resume(&mut self, mut input: Option<Io>) -> Result<Response, Io> {
        loop {
            match &mut self.state {
                State::SendRequest(write) => {
                    if let Err(io) = write.resume(input.take()) {
                        debug!("need to send request");
                        return Err(io);
                    }

                    let read = Read::default();
                    self.state = State::ReceiveResponse(read);
                }
                State::ReceiveResponse(read) => {
                    let output = match read.resume(input.take()) {
                        Ok(output) => output,
                        Err(io) => {
                            debug!("need to receive response chunk");
                            return Err(io);
                        }
                    };

                    let bytes = output.bytes();

                    match memrchr(b'\n', bytes) {
                        Some(n) => {
                            self.response.extend(&bytes[..n]);
                            let response = serde_json::from_slice(&self.response).unwrap();
                            debug!("got complete response: {response:?}");
                            break Ok(response);
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
