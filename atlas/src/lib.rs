//! ATLAS status information.
//!
//! ATLAS is a collection of remote terrestrial LiDAR systems. As of this writing, there are two
//! ATLAS systems, one on the south and one on the north side of the Helheim Glacier in southeast
//! Greenland.
//!
//! ATLAS systems transmit regularly-scheduled messages called "heartbeats," which contain system
//! status information.

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
