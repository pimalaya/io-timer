use io_stream::io::StreamIo;

use crate::{client::coroutines::send::SendRequestResult, Request};

use super::send::SendRequest;

#[derive(Debug)]
pub struct PauseTimer {
    send: SendRequest,
}

impl PauseTimer {
    pub fn new() -> Self {
        let send = SendRequest::new(Request::Pause);
        Self { send }
    }

    pub fn resume(&mut self, arg: Option<StreamIo>) -> SendRequestResult {
        self.send.resume(arg)
    }
}
