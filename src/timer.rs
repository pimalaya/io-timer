//! Pure timer state machine.
//!
//! The [`Timer`] struct is I/O-free: it never reads the clock itself.
//! Methods that need the current time accept `now: u64` (Unix epoch
//! seconds) as a parameter, which the caller obtains via the
//! [`TimeNow`] coroutine or directly from a runtime.
//!
//! [`TimeNow`]: crate::coroutines::now::TimeNow

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

/// Controls how many full loops the timer runs before stopping.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerLoop {
    /// The timer loops indefinitely and never stops by itself.
    ///
    /// The only way to stop such a timer is via [`Timer::stop`].
    #[default]
    Infinite,
    /// The timer stops automatically after the given number of loops.
    Fixed(usize),
}

impl From<usize> for TimerLoop {
    fn from(count: usize) -> Self {
        if count == 0 {
            Self::Infinite
        } else {
            Self::Fixed(count)
        }
    }
}

/// A single step in the timer lifecycle, identified by a name and a
/// duration in seconds.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TimerCycle {
    /// The name of this cycle.
    pub name: String,
    /// Remaining seconds in this cycle.
    ///
    /// From the *configuration* perspective this is the total cycle
    /// duration; from the *running timer* perspective it is the time
    /// remaining before the cycle ends.
    pub duration: usize,
}

impl TimerCycle {
    /// Creates a new cycle with the given name and duration.
    pub fn new(name: impl ToString, duration: usize) -> Self {
        Self {
            name: name.to_string(),
            duration,
        }
    }
}

/// The ordered list of cycles that a timer runs through.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TimerCycles(Vec<TimerCycle>);

impl<T: IntoIterator<Item = TimerCycle>> From<T> for TimerCycles {
    fn from(cycles: T) -> Self {
        Self(cycles.into_iter().collect())
    }
}

impl Deref for TimerCycles {
    type Target = Vec<TimerCycle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TimerCycles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The current state of a timer.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerState {
    /// The timer is running.
    Running,
    /// The timer has been paused.
    Paused,
    /// The timer is not running.
    #[default]
    Stopped,
}

/// An event emitted by a timer during its lifecycle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerEvent {
    /// The timer started.
    Started,
    /// The timer began the given cycle.
    Began(TimerCycle),
    /// The timer is running the given cycle (periodic tick).
    Running(TimerCycle),
    /// The remaining duration was manually set.
    Set(TimerCycle),
    /// The timer was paused at the given cycle.
    Paused(TimerCycle),
    /// The timer was resumed at the given cycle.
    Resumed(TimerCycle),
    /// The timer ended the given cycle.
    Ended(TimerCycle),
    /// The timer stopped.
    Stopped,
}

/// Timer configuration: cycle definitions and loop count.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TimerConfig {
    /// The ordered list of timer cycles.
    pub cycles: TimerCycles,
    /// How many full loops the timer should run.
    pub cycles_count: TimerLoop,
}

impl TimerConfig {
    fn first_cycle(&self) -> TimerCycle {
        self.cycles
            .first()
            .cloned()
            .expect("timer config must have at least one cycle")
    }
}

/// An I/O-free timer state machine.
///
/// All methods that depend on the current time accept `now: u64`
/// (seconds since the Unix epoch) rather than reading the clock
/// internally. Obtain `now` via the [`TimeNow`] coroutine or via the
/// server-side [`TimerRequestHandle`] coroutine.
///
/// [`TimeNow`]: crate::coroutines::now::TimeNow
/// [`TimerRequestHandle`]: crate::coroutines::server::TimerRequestHandle
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Timer {
    /// The timer configuration.
    pub config: TimerConfig,
    /// The current timer state.
    pub state: TimerState,
    /// The current cycle (with remaining duration).
    pub cycle: TimerCycle,
    /// The configured loop count, decremented as loops complete.
    pub cycles_count: TimerLoop,
    /// Unix epoch seconds at which the timer was last started or
    /// resumed. `None` when the timer is stopped or paused.
    pub started_at: Option<u64>,
    /// Accumulated elapsed seconds from previous runs (before the
    /// last pause or stop).
    pub elapsed: usize,
}

impl Timer {
    /// Creates a new timer from the given configuration.
    ///
    /// # Panics
    ///
    /// Panics if `config` has no cycles.
    pub fn new(config: TimerConfig) -> Self {
        let cycle = config.first_cycle();
        let cycles_count = config.cycles_count.clone();
        Self {
            config,
            cycle,
            cycles_count,
            ..Default::default()
        }
    }

    /// Returns the total elapsed seconds since the timer last started
    /// or resumed, plus any previously accumulated elapsed time.
    pub fn elapsed(&self, now: u64) -> usize {
        let running = self
            .started_at
            .map(|s| now.saturating_sub(s) as usize)
            .unwrap_or(0);
        running + self.elapsed
    }

    /// Advances the timer by one tick and returns any events that
    /// fired.
    ///
    /// Has no effect when the timer is paused or stopped.
    pub fn update(&mut self, now: u64) -> impl IntoIterator<Item = TimerEvent> {
        let mut events = Vec::with_capacity(3);

        if let TimerState::Running = self.state {
            let mut elapsed = self.elapsed(now);

            let (cycles, total_duration) = self.config.cycles.iter().cloned().fold(
                (Vec::new(), 0),
                |(mut cycles, mut sum), mut cycle| {
                    cycle.duration += sum;
                    sum = cycle.duration;
                    cycles.push(cycle);
                    (cycles, sum)
                },
            );

            if let TimerLoop::Fixed(cycles_count) = self.cycles_count {
                if elapsed >= total_duration * cycles_count {
                    self.state = TimerState::Stopped;
                    return events;
                }
            }

            elapsed %= total_duration;

            let last_cycle = cycles[cycles.len() - 1].clone();
            let next_cycle = cycles
                .into_iter()
                .fold(None, |next_cycle, mut cycle| match next_cycle {
                    None if elapsed < cycle.duration => {
                        cycle.duration -= elapsed;
                        Some(cycle)
                    }
                    _ => next_cycle,
                })
                .unwrap_or(last_cycle);

            events.push(TimerEvent::Running(self.cycle.clone()));

            if self.cycle.name != next_cycle.name {
                let mut prev_cycle = self.cycle.clone();
                prev_cycle.duration = 0;
                events.push(TimerEvent::Ended(prev_cycle));
                events.push(TimerEvent::Began(next_cycle.clone()));
            }

            self.cycle = next_cycle;
        }

        events
    }

    /// Starts the timer from the first configured cycle.
    ///
    /// Has no effect if the timer is already running or paused.
    pub fn start(&mut self, now: u64) -> impl IntoIterator<Item = TimerEvent> {
        let mut events = Vec::with_capacity(2);

        if matches!(self.state, TimerState::Stopped) {
            self.state = TimerState::Running;
            self.cycle = self.config.first_cycle();
            self.cycles_count = self.config.cycles_count.clone();
            self.started_at = Some(now);
            self.elapsed = 0;
            events.push(TimerEvent::Started);
            events.push(TimerEvent::Began(self.cycle.clone()));
        }

        events
    }

    /// Sets the remaining duration of the current cycle to
    /// `duration_secs`.
    pub fn set(&mut self, duration_secs: usize) -> impl IntoIterator<Item = TimerEvent> {
        self.cycle.duration = duration_secs;
        [TimerEvent::Set(self.cycle.clone())]
    }

    /// Pauses the timer, saving the elapsed time.
    ///
    /// Has no effect if the timer is not running.
    pub fn pause(&mut self, now: u64) -> impl IntoIterator<Item = TimerEvent> {
        if matches!(self.state, TimerState::Running) {
            self.elapsed = self.elapsed(now);
            self.started_at = None;
            self.state = TimerState::Paused;
            Some(TimerEvent::Paused(self.cycle.clone()))
        } else {
            None
        }
    }

    /// Resumes the timer from where it was paused.
    ///
    /// Has no effect if the timer is not paused.
    pub fn resume(&mut self, now: u64) -> impl IntoIterator<Item = TimerEvent> {
        if matches!(self.state, TimerState::Paused) {
            self.state = TimerState::Running;
            self.started_at = Some(now);
            Some(TimerEvent::Resumed(self.cycle.clone()))
        } else {
            None
        }
    }

    /// Stops the timer and resets it to the initial state.
    ///
    /// Has no effect if the timer is not running.
    pub fn stop(&mut self) -> impl IntoIterator<Item = TimerEvent> {
        let mut events = Vec::with_capacity(2);

        if matches!(self.state, TimerState::Running) {
            self.state = TimerState::Stopped;
            events.push(TimerEvent::Ended(self.cycle.clone()));
            events.push(TimerEvent::Stopped);
            self.cycle = self.config.first_cycle();
            self.cycles_count = self.config.cycles_count.clone();
            self.started_at = None;
            self.elapsed = 0;
        }

        events
    }
}

/// A command sent to a timer server.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerRequest {
    /// Return the current timer state without modifying it.
    Get,
    /// Start the timer.
    Start,
    /// Stop the timer.
    Stop,
    /// Pause the timer.
    Pause,
    /// Resume the timer.
    Resume,
    /// Advance the timer by one tick.
    Update,
    /// Set the remaining duration of the current cycle.
    Set(usize),
}

/// A response from a timer server.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimerResponse {
    /// The current timer state (reply to [`TimerRequest::Get`]).
    Timer(Timer),
    /// Events emitted by the timer as a result of a command.
    Events(Vec<TimerEvent>),
}

impl Eq for Timer {}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.cycle == other.cycle
            && self.started_at == other.started_at
            && self.elapsed == other.elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testing_timer() -> Timer {
        Timer {
            config: TimerConfig {
                cycles: TimerCycles::from([
                    TimerCycle::new("a", 3),
                    TimerCycle::new("b", 2),
                    TimerCycle::new("c", 1),
                ]),
                ..Default::default()
            },
            state: TimerState::Running,
            cycle: TimerCycle::new("a", 3),
            started_at: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn running_infinite_timer() {
        let mut timer = testing_timer();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));

        timer.update(2);
        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 1));

        timer.update(3);
        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("b", 2));

        timer.update(5);
        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("c", 1));

        timer.update(6);
        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));
    }

    #[test]
    fn running_timer_events() {
        let mut timer = testing_timer();
        let mut events = Vec::new();

        events.extend(timer.update(1));
        events.extend(timer.update(2));
        events.extend(timer.update(3));
        events.extend(timer.update(4));

        assert_eq!(
            events,
            vec![
                TimerEvent::Running(TimerCycle::new("a", 3)),
                TimerEvent::Running(TimerCycle::new("a", 2)),
                TimerEvent::Running(TimerCycle::new("a", 1)),
                TimerEvent::Ended(TimerCycle::new("a", 0)),
                TimerEvent::Began(TimerCycle::new("b", 2)),
                TimerEvent::Running(TimerCycle::new("b", 2)),
            ]
        );
    }

    #[test]
    fn paused_timer_not_impacted_by_update() {
        let mut timer = testing_timer();
        timer.state = TimerState::Paused;
        let prev_timer = timer.clone();
        timer.update(10);
        assert_eq!(prev_timer, timer);
    }

    #[test]
    fn stopped_timer_not_impacted_by_update() {
        let mut timer = testing_timer();
        timer.state = TimerState::Stopped;
        let prev_timer = timer.clone();
        timer.update(10);
        assert_eq!(prev_timer, timer);
    }

    #[test]
    fn timer_lifecycle() {
        let mut timer = Timer::new(TimerConfig {
            cycles: TimerCycles::from([
                TimerCycle::new("a", 3),
                TimerCycle::new("b", 2),
                TimerCycle::new("c", 1),
            ]),
            ..Default::default()
        });

        let mut events = Vec::new();

        assert_eq!(timer.state, TimerState::Stopped);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));

        events.extend(timer.start(0));
        events.extend(timer.set(21));

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 21));

        events.extend(timer.pause(0));

        assert_eq!(timer.state, TimerState::Paused);
        assert_eq!(timer.cycle, TimerCycle::new("a", 21));

        events.extend(timer.resume(0));

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 21));

        events.extend(timer.stop());

        assert_eq!(timer.state, TimerState::Stopped);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));

        assert_eq!(
            events,
            vec![
                TimerEvent::Started,
                TimerEvent::Began(TimerCycle::new("a", 3)),
                TimerEvent::Set(TimerCycle::new("a", 21)),
                TimerEvent::Paused(TimerCycle::new("a", 21)),
                TimerEvent::Resumed(TimerCycle::new("a", 21)),
                TimerEvent::Ended(TimerCycle::new("a", 21)),
                TimerEvent::Stopped,
            ]
        );
    }
}
