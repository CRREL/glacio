//! Reconstructed Sutron messages, which are built from one or more packets.
//!
//! # Examples
//!
//! Use a `Reassembler` to build messages from packets that may not come in order:
//!
//! ```
//! use sutron::{message::Reassembler, Packet};
//! let packet_a = Packet::new(b"1,42,0,2:a").unwrap();
//! let packet_standalone = Packet::new(b"0test message").unwrap();
//! let packet_b = Packet::new(b"1,42,1:b").unwrap();
//! let mut reassembler = Reassembler::new();
//!
//! assert_eq!(None, reassembler.add(packet_a));
//!
//! let message_standalone = reassembler.add(packet_standalone).unwrap();
//! assert_eq!(b"test message".to_vec(), message_standalone.data);
//!
//! let message_multipacket = reassembler.add(packet_b).unwrap();
//! assert_eq!(b"ab".to_vec(), message_multipacket.data);
//! ```
//!
//! If you know the packets are in order, you can create a message directly:
//!
//! ```
//! use sutron::{Packet, Message};
//! let packet_a = Packet::new(b"1,42,0,2:a").unwrap();
//! let packet_b = Packet::new(b"1,42,1:b").unwrap();
//! let message = Message::new(vec![packet_a, packet_b]).unwrap();
//! ```

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use Packet;

/// A message sent from a Sutron system.
#[derive(Debug, PartialEq)]
pub struct Message {
    /// The raw bytes that make up the message.
    pub data: Vec<u8>,

    /// The date and time the message began.
    ///
    /// Usually derived from the start packet's SBD message's time of session.
    pub datetime: Option<DateTime<Utc>>,

    /// The packets, in order, that were used to create the message.
    pub packets: Vec<Packet>,
}

/// Reassembles packets into messages.
#[derive(Debug, Default)]
pub struct Reassembler {
    packet_map: HashMap<u8, Vec<Packet>>,
    recycle_bin: Vec<Packet>,
}

/// Errors associated with creating messages.
#[derive(Debug, Fail)]
pub enum Error {
    /// The length of the data does not match the length advertised in the header.
    #[fail(
        display = "the size was advertised as {} bytes but was actually {} bytes",
        sub_header,
        actual
    )]
    DataLengthMismatch {
        /// The size as advertised in the packet sub-header.
        sub_header: usize,

        /// The actual length of the data.
        actual: usize,
    },

    /// There are no packets from which to create this message.
    #[fail(display = "there are no packets from which to create the message")]
    NoPackets,

    /// There is no total bytes field in the first packet of the extended message.
    #[fail(display = "there is no total bytes field in the sub header of the first packet")]
    NoTotalBytes(Packet),
}

impl Message {
    /// Creates a new message from a non-extended sbd message in a file.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Message;
    /// let message = Message::from_path("fixtures/self-timed.sbd").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Message, ::failure::Error> {
        ::sbd::mo::Message::from_path(path)
            .map_err(::failure::Error::from)
            .and_then(|message| Packet::from_message(message))
            .and_then(|packet| Message::new(vec![packet]).map_err(::failure::Error::from))
    }

    /// Creates a new message from one or more packets.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::{Packet, Message};
    /// let packet = Packet::new(b"0self-timed").unwrap();
    /// let message = Message::new(vec![packet]).unwrap();
    /// ```
    pub fn new(mut packets: Vec<Packet>) -> Result<Message, Error> {
        if packets.is_empty() {
            return Err(Error::NoPackets);
        }
        let sub_header = &packets[0].sub_header();
        let datetime = packets[0].datetime();
        if let Some(sub_header) = sub_header {
            if let Some(total_bytes) = sub_header.total_bytes {
                let data = packets.iter().map(|p| p.data()).fold(
                    Vec::new(),
                    |mut vec: Vec<u8>, bytes| {
                        vec.extend(bytes);
                        vec
                    },
                );
                if data.len() == total_bytes {
                    Ok(Message {
                        data: data,
                        datetime: datetime,
                        packets: packets,
                    })
                } else {
                    Err(Error::DataLengthMismatch {
                        sub_header: total_bytes,
                        actual: data.len(),
                    })
                }
            } else {
                Err(Error::NoTotalBytes(packets.remove(0)))
            }
        } else {
            Ok(Message {
                data: packets[0].data().to_vec(),
                datetime: datetime,
                packets: packets,
            })
        }
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Message {
        Message {
            data: data,
            datetime: None,
            packets: Vec::new(),
        }
    }
}

impl Reassembler {
    /// Creates a new reassembler.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::message::Reassembler;
    /// let reassembler = Reassembler::new();
    /// ```
    pub fn new() -> Reassembler {
        Reassembler::default()
    }

    /// Adds a new packet to the reassembler and returns a message if one was completed.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::{Packet, message::Reassembler};
    /// let mut reassembler = Reassembler::new();
    /// let message = reassembler.add(Packet::new(b"0test message").unwrap()).unwrap();
    /// assert_eq!(b"test message".as_ref(), message.data.as_slice());
    /// ```
    pub fn add(&mut self, packet: Packet) -> Option<Message> {
        if let Some(sub_header) = packet.sub_header() {
            let entry = self
                .packet_map
                .entry(sub_header.id)
                .or_insert_with(Vec::new);
            if packet.is_start_packet() {
                self.recycle_bin.extend(entry.drain(..))
            }
            entry.push(packet);
            if let Ok(message) = Message::new(entry.clone()) {
                entry.clear();
                Some(message)
            } else {
                None
            }
        } else {
            Message::new(vec![packet]).ok()
        }
    }

    /// Returns a reference to all messages that have been discarded by this reassembler.
    ///
    /// Messages are discarded when a new start packet comes in with the same id.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::{Packet, message::Reassembler};
    /// let packet = Packet::new(b"1,42,0,3:a").unwrap();
    /// let mut reassembler = Reassembler::new();
    /// assert_eq!(None, reassembler.add(packet.clone()));
    /// assert!(reassembler.recycle_bin().is_empty());
    /// assert_eq!(None, reassembler.add(packet.clone()));
    /// assert_eq!([packet], reassembler.recycle_bin());
    /// ```
    pub fn recycle_bin(&self) -> &[Packet] {
        &self.recycle_bin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures() {
        let mut reassembler = Reassembler::new();
        assert!(
            reassembler
                .add(Packet::from_path("fixtures/self-timed.sbd").unwrap())
                .is_some()
        );
        assert!(
            reassembler
                .add(Packet::from_path("fixtures/self-timed-extended-0.sbd").unwrap())
                .is_none()
        );
        assert!(
            reassembler
                .add(Packet::from_path("fixtures/self-timed-extended-1.sbd").unwrap())
                .is_some()
        );
    }

    #[test]
    fn one_message() {
        let packet = Packet::new(b"0self-timed message").unwrap();
        let mut reassembler = Reassembler::new();
        let message = reassembler.add(packet).unwrap();
        assert_eq!(b"self-timed message".as_ref(), message.data.as_slice());
    }

    #[test]
    fn one_message_two_packets() {
        let packet_a = Packet::new(b"1,42,0,2:a").unwrap();
        let packet_b = Packet::new(b"1,42,1:b").unwrap();
        let mut reassembler = Reassembler::new();
        assert_eq!(None, reassembler.add(packet_a));
        let message = reassembler.add(packet_b).unwrap();
        assert_eq!(b"ab".as_ref(), message.data.as_slice());
    }

    #[test]
    fn two_messages_interleaved() {
        let packet_a = Packet::new(b"1,42,0,2:a").unwrap();
        let packet_b = Packet::new(b"1,42,1:b").unwrap();
        let packet_c = Packet::new(b"1,43,0,2:c").unwrap();
        let packet_d = Packet::new(b"1,43,1:d").unwrap();
        let mut reassembler = Reassembler::new();
        assert_eq!(None, reassembler.add(packet_a));
        assert_eq!(None, reassembler.add(packet_c));
        let message = reassembler.add(packet_b).unwrap();
        assert_eq!(b"ab".as_ref(), message.data.as_slice());
        let message = reassembler.add(packet_d).unwrap();
        assert_eq!(b"cd".as_ref(), message.data.as_slice());
    }

    #[test]
    fn one_message_with_reset() {
        let packet_a = Packet::new(b"1,42,0,2:a").unwrap();
        let packet_b = Packet::new(b"1,42,1:b").unwrap();
        let packet_c = Packet::new(b"1,42,0,2:c").unwrap();
        let mut reassembler = Reassembler::new();
        assert_eq!(None, reassembler.add(packet_a.clone()));
        assert_eq!(None, reassembler.add(packet_c));
        let message = reassembler.add(packet_b).unwrap();
        assert_eq!(b"cb".as_ref(), message.data.as_slice());
        assert_eq!([packet_a], reassembler.recycle_bin());
    }

    #[test]
    fn one_message_too_long() {
        let packet_a = Packet::new(b"1,42,0,2:a").unwrap();
        let packet_b = Packet::new(b"1,42,1:bc").unwrap();
        let packet_c = Packet::new(b"1,42,0,3:c").unwrap();
        let mut reassembler = Reassembler::new();
        assert_eq!(None, reassembler.add(packet_a));
        assert_eq!(None, reassembler.add(packet_b.clone()));
        assert_eq!(None, reassembler.add(packet_c));
        let message = reassembler.add(packet_b).unwrap();
        assert_eq!(b"cbc".as_ref(), message.data.as_slice());
    }

    #[test]
    fn message_from_vec() {
        let data = vec![0u8, 42u8];
        let message = Message::from(data);
        assert_eq!(vec![0u8, 42u8], message.data);
        assert_eq!(None, message.datetime);
        assert_eq!(Vec::<Packet>::new(), message.packets);
    }
}
