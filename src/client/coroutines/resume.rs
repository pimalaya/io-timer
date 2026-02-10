use io_stream::io::StreamIo;

use crate::{client::coroutines::send::SendRequestResult, Request};

use super::send::SendRequest;

#[derive(Debug)]
pub struct ResumeTimer {
    send: SendRequest,
}

impl ResumeTimer {
    pub fn new() -> Self {
        let send = SendRequest::new(Request::Resume);
        Self { send }
    }

    pub fn resume(&mut self, arg: Option<StreamIo>) -> SendRequestResult {
        self.send.resume(arg)
    }
}
