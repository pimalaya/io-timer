#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod coroutines;
pub mod io;
pub mod runtimes;
#[cfg(feature = "timer")]
pub mod timer;
