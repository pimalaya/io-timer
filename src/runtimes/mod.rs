//! Collection of time runtimes.
//!
//! A runtime contains all the I/O logic, and is responsible for
//! processing [`TimeInput`] requests emitted by [coroutines] and
//! returning the corresponding [`TimeOutput`].
//!
//! If you miss a runtime matching your requirements, you can easily
//! implement your own by taking example on the existing ones. PRs are
//! welcomed!
//!
//! [`TimeInput`]: crate::io::TimeInput
//! [`TimeOutput`]: crate::io::TimeOutput
//! [coroutines]: crate::coroutines

#[cfg(feature = "std")]
pub mod std;
