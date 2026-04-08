//! Sleep for one second using the [`TimeSleep`] coroutine, then sleep
//! until the next second boundary using [`TimeSleepUntil`], both with
//! the standard blocking runtime.

use std::time::Instant;

use io_time::{
    coroutines::{
        now::{TimeNow, TimeNowResult},
        sleep::{TimeSleep, TimeSleepResult},
        sleep_until::{TimeSleepUntil, TimeSleepUntilResult},
    },
    runtimes::std::handle,
};

fn main() {
    // --- TimeSleep: sleep for a relative duration ---

    println!("Sleeping for 1s…");
    let before = Instant::now();

    let mut sleep = TimeSleep::new(1);
    let mut arg = None;

    loop {
        match sleep.resume(arg.take()) {
            TimeSleepResult::Ok => break,
            TimeSleepResult::Io { input } => arg = Some(handle(input).unwrap()),
            TimeSleepResult::Err { err } => panic!("{err}"),
        }
    }

    println!("Woke up after {:.3}s\n", before.elapsed().as_secs_f64());

    // --- TimeSleepUntil: sleep until the next second boundary ---

    let mut now = TimeNow::new();
    let mut arg = None;

    let secs = loop {
        match now.resume(arg.take()) {
            TimeNowResult::Ok { secs, .. } => break secs,
            TimeNowResult::Io { input } => arg = Some(handle(input).unwrap()),
            TimeNowResult::Err { err } => panic!("{err}"),
        }
    };

    let target = secs + 1;
    println!("Sleeping until Unix epoch {target}s…");
    let before = Instant::now();

    let mut sleep_until = TimeSleepUntil::new(target);
    let mut arg = None;

    loop {
        match sleep_until.resume(arg.take()) {
            TimeSleepUntilResult::Ok => break,
            TimeSleepUntilResult::Io { input } => arg = Some(handle(input).unwrap()),
            TimeSleepUntilResult::Err { err } => panic!("{err}"),
        }
    }

    println!("Woke up after {:.3}s", before.elapsed().as_secs_f64());
}
