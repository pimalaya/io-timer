use io_time::{
    coroutines::now::{TimeNow, TimeNowResult},
    io::{TimeInput, TimeOutput},
};

#[test]
fn emits_now_request() {
    let mut coroutine = TimeNow::new();

    match coroutine.resume(None) {
        TimeNowResult::Io {
            input: TimeInput::Now,
        } => {}
        other => panic!("expected Io {{ TimeInput::Now }}, got {other:?}"),
    }
}

#[test]
fn returns_time_on_valid_arg() {
    let mut coroutine = TimeNow::new();

    match coroutine.resume(Some(TimeOutput::Now {
        secs: 1_700_000_000,
        nanos: 42,
    })) {
        TimeNowResult::Ok {
            secs: 1_700_000_000,
            nanos: 42,
        } => {}
        other => panic!("expected Ok {{ secs, nanos }}, got {other:?}"),
    }
}

#[test]
fn returns_err_on_invalid_arg() {
    let mut coroutine = TimeNow::new();

    match coroutine.resume(Some(TimeOutput::Slept)) {
        TimeNowResult::Err { .. } => {}
        other => panic!("expected Err, got {other:?}"),
    }
}
