//! ATLAS is a collection of remote terrestrial LiDAR systems in southeast Greenland.
//!
//! There are two ATLAS systems, ATLAS South and ATLAS North. Both ATLAS systems transmit
//! regularly-scheduled messages called "heartbeats," which contain system status information. This
//! crate parses those messages into serde-able data structures.
//!
//! # Examples
//!
//! Heartbeat messages come in as one or more Iridium SBD messages. A heartbeat can be created
//! directly from SBD messages on the filesystem:
//!
//! ```
//! use atlas::Heartbeat;
//! let heartbeat = Heartbeat::from_paths(&vec![
//!     "fixtures/sbd/181002_050602.sbd",
//!     "fixtures/sbd/181002_050622.sbd",
//! ]);
//! ```

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
