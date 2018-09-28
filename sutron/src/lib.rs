//! Reconstruct messages from Sutron-constructed Iridium SBD packets.
//!
//! # Background
//!
//! Sutron data loggers can transmit information using [Iridium
//! SBD](https://github.com/gadomski/sbd-rs) messages. Each SBD message can only be a certain
//! number of bytes, so Sutron developed its own protocol for transmitting more information than
//! can fit in one SBD message. See Appendix B of `references/sutron-iridium.pdf` in the source
//! repository of this code for a description of this protocol.

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

/// Reassembles Sutron messages from SBD messages.
///
/// This is the simplest (and dumbest) way to do this. All exceptional conditions are discarded
/// silently, and only successfully reconstructed messages are returned. For finer-grained control,
/// use `message::Reassembler`.
///
/// # Examples
///
/// ```
/// # extern crate sbd;
/// # extern crate sutron;
/// # fn main() {
/// use sbd::mo::Message;
///
/// let sbd_messages = vec![
///     Message::from_path("fixtures/self-timed-extended-0.sbd").unwrap(),
///     Message::from_path("fixtures/forced.sbd").unwrap(),
///     Message::from_path("fixtures/self-timed-extended-1.sbd").unwrap(),
/// ];
/// let messages = sutron::reassemble(sbd_messages.into_iter());
/// assert_eq!(2, messages.len());
/// # }
/// ```
pub fn reassemble<I: Iterator<Item = sbd::mo::Message>>(iter: I) -> Vec<Message> {
    use message::Reassembler;
    let mut reassembler = Reassembler::new();
    iter.filter_map(|sbd_message| {
        Packet::from_message(sbd_message)
            .ok()
            .and_then(|packet| reassembler.add(packet))
    }).collect()
}
