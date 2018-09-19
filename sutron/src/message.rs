//! Full messages from the data logger, which may or may not be broken up over multiple packets.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
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
}

#[derive(Debug)]
enum Error {
    DataLengthMismatch { sub_header: usize, actual: usize },
    NoPackets,
    NoTotalBytes(Packet),
}

impl Message {
    fn new(packets: &[Packet]) -> Result<Message, Error> {
        if packets.is_empty() {
            return Err(Error::NoPackets);
        }
        let start_packet = &packets[0];
        if let Some(sub_header) = start_packet.sub_header() {
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
                        datetime: start_packet.datetime(),
                        packets: packets.to_vec(),
                    })
                } else {
                    Err(Error::DataLengthMismatch {
                        sub_header: total_bytes,
                        actual: data.len(),
                    })
                }
            } else {
                Err(Error::NoTotalBytes(start_packet.clone()))
            }
        } else {
            Ok(Message {
                data: start_packet.data().to_vec(),
                datetime: start_packet.datetime(),
                packets: packets.to_vec(),
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

    /// Adds a new packet to the reassembler, and returns a message if one was completed.
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
                entry.clear();
            }
            entry.push(packet);
            if let Ok(message) = Message::new(entry) {
                entry.clear();
                Some(message)
            } else {
                None
            }
        } else {
            Message::new(&[packet]).ok()
        }
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
        assert_eq!(None, reassembler.add(packet_a));
        assert_eq!(None, reassembler.add(packet_c));
        let message = reassembler.add(packet_b).unwrap();
        assert_eq!(b"cb".as_ref(), message.data.as_slice());
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
