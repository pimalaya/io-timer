use io_stream::Io;

use crate::{Request, Response};

use super::SendRequest;

#[derive(Debug)]
pub struct StopTimer {
    send: SendRequest,
}

impl StopTimer {
    pub fn new() -> Self {
        let send = SendRequest::new(Request::Stop);
        Self { send }
    }

    pub fn resume(&mut self, input: Option<Io>) -> Result<Response, Io> {
        self.send.resume(input)
    }
}
