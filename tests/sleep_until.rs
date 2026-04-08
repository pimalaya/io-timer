use io_time::{
    coroutines::sleep_until::{TimeSleepUntil, TimeSleepUntilResult},
    io::{TimeInput, TimeOutput},
};

#[test]
fn emits_sleep_until_request() {
    let mut coroutine = TimeSleepUntil::new(1_700_000_001);

    match coroutine.resume(None) {
        TimeSleepUntilResult::Io {
            input: TimeInput::SleepUntil {
                timestamp: 1_700_000_001,
            },
        } => {}
        other => panic!("expected Io {{ TimeInput::SleepUntil {{ timestamp }} }}, got {other:?}"),
    }
}

#[test]
fn returns_ok_after_slept() {
    let mut coroutine = TimeSleepUntil::new(1_700_000_001);
    coroutine.resume(None);

    match coroutine.resume(Some(TimeOutput::Slept)) {
        TimeSleepUntilResult::Ok => {}
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn returns_err_on_invalid_arg() {
    let mut coroutine = TimeSleepUntil::new(1_700_000_001);

    match coroutine.resume(Some(TimeOutput::Now { secs: 0, nanos: 0 })) {
        TimeSleepUntilResult::Err { .. } => {}
        other => panic!("expected Err, got {other:?}"),
    }
}
