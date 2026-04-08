use io_time::{
    coroutines::sleep::{TimeSleep, TimeSleepResult},
    io::{TimeInput, TimeOutput},
};

#[test]
fn emits_sleep_request() {
    let mut coroutine = TimeSleep::new(5);

    match coroutine.resume(None) {
        TimeSleepResult::Io {
            input: TimeInput::Sleep { secs: 5 },
        } => {}
        other => panic!("expected Io {{ TimeInput::Sleep {{ secs: 5 }} }}, got {other:?}"),
    }
}

#[test]
fn returns_ok_after_slept() {
    let mut coroutine = TimeSleep::new(5);
    coroutine.resume(None);

    match coroutine.resume(Some(TimeOutput::Slept)) {
        TimeSleepResult::Ok => {}
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn returns_err_on_invalid_arg() {
    let mut coroutine = TimeSleep::new(5);

    match coroutine.resume(Some(TimeOutput::Now { secs: 0, nanos: 0 })) {
        TimeSleepResult::Err { .. } => {}
        other => panic!("expected Err, got {other:?}"),
    }
}
