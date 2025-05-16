use io_stream::Io;

use crate::{Request, Response};

use super::SendRequest;

#[derive(Debug)]
pub struct PauseTimer {
    send: SendRequest,
}

impl PauseTimer {
    pub fn new() -> Self {
        let send = SendRequest::new(Request::Pause);
        Self { send }
    }

    pub fn resume(&mut self, input: Option<Io>) -> Result<Response, Io> {
        self.send.resume(input)
    }
}
