//! ATLAS is a collection of remote terrestrial LiDAR systems in southeast Greenland.
//!
//! There are two ATLAS systems, ATLAS South and ATLAS North. Both ATLAS systems transmit
//! regularly-scheduled messages called "heartbeats," which contain system status information. This
//! crate parses those messages into serde-able data structures.
//!
//! # Examples
//!
//! Heartbeat messages come in as one or more Iridium SBD messages. These messages are sent by
//! Sutron data loggers, which append their own header information to the messages. The best way to
//! create heartbeats from raw SBD messages is to use the `sutron` crate:
//!
//! ```
//! # extern crate sutron;
//! # extern crate atlas;
//! # fn main() {
//! use sutron::{Packet, Message};
//! use atlas::Heartbeat;
//! let packet_0 = Packet::from_path("fixtures/sbd/181002_050602.sbd").unwrap();
//! let packet_1 = Packet::from_path("fixtures/sbd/181002_050622.sbd").unwrap();
//! let message = Message::new(vec![packet_0, packet_1]).unwrap();
//! let heartbeat = Heartbeat::new(&message).unwrap();
//! # }
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
