//! Higher-level crate for managing ATLAS status information.
//!
//! Most of this information comes in the form of heartbeats, messages sent from ATLAS systems via
//! Iridium SBD that contain information such as the last scan time, air temperature, etc.

#![deny(missing_docs, missing_debug_implementations, unsafe_code)]

extern crate byteorder;
extern crate chrono;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate sbd;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate sutron;

pub mod heartbeat;
mod site;

pub use heartbeat::Heartbeat;
pub use site::Site;
