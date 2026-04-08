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
    runtimes::std::handle as time_handle,
    timer::{
        Timer, TimerConfig, TimerCycle, TimerCycles, TimerEvent, TimerLoop, TimerRequest,
        TimerResponse, TimerState,
    },
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

fn send(mut stream: UnixStream, request: TimerRequest) -> TimerResponse {
    let mut client = TimerRequestSend::new(request);
    let mut arg = None;

    loop {
        match client.resume(arg.take()) {
            TimerRequestSendResult::Ok { response } => return response,
            TimerRequestSendResult::Io { input } => arg = Some(socket_io(&mut stream, input)),
            TimerRequestSendResult::Err { err } => panic!("client error: {err}"),
        }
    }
}

fn handle(mut stream: UnixStream, mut timer: Timer) -> (Timer, Vec<TimerEvent>) {
    let mut server = TimerRequestHandle::new();
    let mut arg: Option<TimerRequestHandleArg> = None;

    loop {
        match server.resume(&mut timer, arg.take()) {
            TimerRequestHandleResult::Ok { events } => return (timer, events),
            TimerRequestHandleResult::Io { input } => {
                arg = Some(TimerRequestHandleArg::Socket(socket_io(&mut stream, input)));
            }
            TimerRequestHandleResult::TimeIo { input } => {
                arg = Some(TimerRequestHandleArg::Time(time_handle(input).unwrap()));
            }
            TimerRequestHandleResult::Err { err } => panic!("server error: {err}"),
        }
    }
}

fn test_timer() -> Timer {
    Timer::new(TimerConfig {
        cycles: TimerCycles::from([
            TimerCycle::new("Focus", 1500),
            TimerCycle::new("Break", 300),
        ]),
        cycles_count: TimerLoop::Infinite,
    })
}

fn pair() -> (UnixStream, UnixStream) {
    UnixStream::pair().unwrap()
}

#[test]
fn get_returns_stopped_timer() {
    let (client_stream, server_stream) = pair();
    let timer = test_timer();

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Get);
    let (_, events) = server.join().unwrap();

    assert!(matches!(response, TimerResponse::Timer(_)));
    assert!(events.is_empty());

    if let TimerResponse::Timer(t) = response {
        assert_eq!(t.state, TimerState::Stopped);
    }
}

#[test]
fn start_returns_started_and_began_events() {
    let (client_stream, server_stream) = pair();
    let timer = test_timer();

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Start);
    let (_, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    assert_eq!(resp_events.len(), 2);
    assert_eq!(resp_events[0], TimerEvent::Started);
    assert!(matches!(resp_events[1], TimerEvent::Began(_)));
    assert_eq!(resp_events, events);
}

#[test]
fn start_on_already_running_timer_is_noop() {
    let (client_stream, server_stream) = pair();
    let mut timer = test_timer();
    timer.start(0).into_iter().for_each(drop);

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Start);
    let (_, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    assert!(resp_events.is_empty());
    assert!(events.is_empty());
}

#[test]
fn stop_running_timer_returns_ended_and_stopped_events() {
    let (client_stream, server_stream) = pair();
    let mut timer = test_timer();
    timer.start(0).into_iter().for_each(drop);

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Stop);
    let (_, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    assert_eq!(resp_events.len(), 2);
    assert!(matches!(resp_events[0], TimerEvent::Ended(_)));
    assert_eq!(resp_events[1], TimerEvent::Stopped);
    assert_eq!(resp_events, events);
}

#[test]
fn stop_on_stopped_timer_is_noop() {
    let (client_stream, server_stream) = pair();
    let timer = test_timer();

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Stop);
    let (_, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    assert!(resp_events.is_empty());
    assert!(events.is_empty());
}

#[test]
fn pause_running_timer_returns_paused_event() {
    let (client_stream, server_stream) = pair();
    let mut timer = test_timer();
    timer.start(0).into_iter().for_each(drop);

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Pause);
    let (_, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    assert_eq!(resp_events.len(), 1);
    assert!(matches!(resp_events[0], TimerEvent::Paused(_)));
    assert_eq!(resp_events, events);
}

#[test]
fn resume_paused_timer_returns_resumed_event() {
    let (client_stream, server_stream) = pair();
    let mut timer = test_timer();
    timer.start(0).into_iter().for_each(drop);
    timer.pause(0).into_iter().for_each(drop);

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Resume);
    let (_, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    assert_eq!(resp_events.len(), 1);
    assert!(matches!(resp_events[0], TimerEvent::Resumed(_)));
    assert_eq!(resp_events, events);
}

#[test]
fn set_updates_cycle_duration() {
    let (client_stream, server_stream) = pair();
    let timer = test_timer();

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Set(60));
    let (updated_timer, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    assert_eq!(resp_events.len(), 1);
    assert!(matches!(resp_events[0], TimerEvent::Set(_)));
    assert_eq!(updated_timer.cycle.duration, 60);
    assert_eq!(resp_events, events);
}

#[test]
fn update_on_running_timer_returns_running_event() {
    let (client_stream, server_stream) = pair();
    let mut timer = test_timer();
    timer.start(0).into_iter().for_each(drop);

    let server = thread::spawn(move || handle(server_stream, timer));
    let response = send(client_stream, TimerRequest::Update);
    let (_, events) = server.join().unwrap();

    let resp_events = match response {
        TimerResponse::Events(e) => e,
        other => panic!("expected Events, got {other:?}"),
    };

    // At least one Running event
    assert!(!resp_events.is_empty());
    assert!(matches!(resp_events[0], TimerEvent::Running(_)));
    assert_eq!(resp_events, events);
}
