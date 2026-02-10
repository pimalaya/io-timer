use io_stream::io::StreamIo;

use crate::{client::coroutines::send::SendRequestResult, Request};

use super::send::SendRequest;

#[derive(Debug)]
pub struct GetTimer {
    send: SendRequest,
}

impl GetTimer {
    pub fn new() -> Self {
        let send = SendRequest::new(Request::Get);
        Self { send }
    }

    pub fn resume(&mut self, arg: Option<StreamIo>) -> SendRequestResult {
        self.send.resume(arg)
    }
}
