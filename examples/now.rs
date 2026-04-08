//! Get the current time using the [`TimeNow`] coroutine and the
//! standard blocking runtime.

use io_time::{
    coroutines::now::{TimeNow, TimeNowResult},
    runtimes::std::handle,
};

fn main() {
    let mut now = TimeNow::new();
    let mut arg = None;

    loop {
        match now.resume(arg.take()) {
            TimeNowResult::Ok { secs, nanos } => {
                println!("{secs}s {nanos}ns since the Unix epoch");
                break;
            }
            TimeNowResult::Io { input } => arg = Some(handle(input).unwrap()),
            TimeNowResult::Err { err } => panic!("{err}"),
        }
    }
}
