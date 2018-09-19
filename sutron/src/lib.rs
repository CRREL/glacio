//! Iridum SBD messages are limited in size, so Sutron breaks up the message and we use this
//! library to reconstruct them.

#![deny(missing_docs, missing_debug_implementations, unsafe_code)]

extern crate byteorder;
extern crate chrono;
#[macro_use]
extern crate failure;
extern crate sbd;

pub mod message;
pub mod packet;

pub use message::Message;
pub use packet::Packet;
