//! End-to-end example: start a timer from a client connected to a
//! server over a Unix socket pair.
//!
//! Both sides run in separate threads over an in-process socket pair.
//! The server drives [`TimerRequestHandle`] and the client drives
//! [`TimerRequestSend`].

use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    thread,
};

use io_socket::io::{SocketInput, SocketOutput};
use io_time::{
    coroutines::{
        client::{TimerRequestSend, TimerRequestSendResult},
        server::{TimerRequestHandle, TimerRequestHandleArg, TimerRequestHandleResult},
    },
    runtimes::std::handle,
    timer::{Timer, TimerConfig, TimerCycle, TimerCycles, TimerLoop},
};

fn socket_io(stream: &mut (impl Read + Write), input: SocketInput) -> SocketOutput {
    match input {
        SocketInput::Read { mut buf } => {
            let n = stream.read(&mut buf).unwrap();
            SocketOutput::Read { buf, n }
        }
        SocketInput::Write { buf } => {
            let n = stream.write(&buf).unwrap();
            SocketOutput::Wrote { buf, n }
        }
    }
}

fn main() {
    let config = TimerConfig {
        cycles: TimerCycles::from([
            TimerCycle::new("Focus", 25 * 60),
            TimerCycle::new("Break", 5 * 60),
        ]),
        cycles_count: TimerLoop::Fixed(4),
    };
    let timer = Timer::new(config);
    let (mut client_stream, mut server_stream) = UnixStream::pair().unwrap();

    // Server thread: handles one request and returns the resulting events.
    let server = thread::spawn(move || {
        let mut timer = timer;
        let mut server = TimerRequestHandle::new();
        let mut arg: Option<TimerRequestHandleArg> = None;

        loop {
            match server.resume(&mut timer, arg.take()) {
                TimerRequestHandleResult::Ok { events } => return events,
                TimerRequestHandleResult::Io { input } => {
                    arg = Some(TimerRequestHandleArg::Socket(socket_io(
                        &mut server_stream,
                        input,
                    )));
                }
                TimerRequestHandleResult::TimeIo { input } => {
                    arg = Some(TimerRequestHandleArg::Time(handle(input).unwrap()));
                }
                TimerRequestHandleResult::Err { err } => panic!("server error: {err}"),
            }
        }
    });

    // Client: send a Start request and wait for the response.
    let mut client = TimerRequestSend::start();
    let mut arg = None;

    let response = loop {
        match client.resume(arg.take()) {
            TimerRequestSendResult::Ok { response } => break response,
            TimerRequestSendResult::Io { input } => {
                arg = Some(socket_io(&mut client_stream, input))
            }
            TimerRequestSendResult::Err { err } => panic!("client error: {err}"),
        }
    };

    let events = server.join().unwrap();

    println!("Response:  {response:?}");
    println!("Events ({}):", events.len());
    for event in &events {
        println!("  {event:?}");
    }
}
