//! Collection of I/O-free, resumable and composable time state
//! machines.
//!
//! Coroutines emit [`TimeInput`] or [`SocketInput`] I/O requests that
//! need to be processed by runtimes in order to continue their
//! progression.
//!
//! [`TimeInput`]: crate::io::TimeInput
//! [`SocketInput`]: io_socket::io::SocketInput

#[cfg(feature = "timer")]
pub mod client;
pub mod now;
#[cfg(feature = "timer")]
pub mod server;
pub mod sleep;
pub mod sleep_until;
