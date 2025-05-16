use std::mem;

use io_stream::{
    coroutines::{Read, Write},
    Io,
};
use log::{debug, trace};
use memchr::memrchr;

use crate::{timer::TimerEvent, Request, Response, Timer};

#[derive(Debug)]
pub enum State {
    ReceiveRequest(Read),
    SendResponse(Write),
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
            state: State::ReceiveRequest(Read::default()),
            request: Vec::new(),
            events: Vec::with_capacity(2),
        }
    }

    pub fn resume(
        &mut self,
        timer: &mut Timer,
        mut input: Option<Io>,
    ) -> Result<impl IntoIterator<Item = TimerEvent>, Io> {
        loop {
            match &mut self.state {
                State::ReceiveRequest(read) => {
                    let output = match read.resume(input.take()) {
                        Ok(output) => output,
                        Err(io) => {
                            debug!("need to receive request chunk");
                            return Err(io);
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

                    let coroutine = Write::new(response.to_vec());
                    self.state = State::SendResponse(coroutine);
                }
                State::SendResponse(write) => {
                    if let Err(io) = write.resume(input.take()) {
                        debug!("need to send response");
                        return Err(io);
                    }

                    let events = mem::take(&mut self.events);
                    debug!("generated {} events to be processed", events.len());
                    trace!("{events:#?}");
                    break Ok(events);
                }
            }
        }
    }
}
