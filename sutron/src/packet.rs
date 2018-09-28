//! Sutron message packets, which correspond roughly one-to-one with Iridium SBD messages.

use chrono::{DateTime, Utc};
use failure::Error as FailureError;
use sbd::mo::Message;
use std::path::Path;

const LOOK_TO_NEXT_BYTE_FOR_MEANING: u8 = b'~';
const SUB_HEADER_TERMINATOR: u8 = b':';

/// A single SBD message, that can be part of or a whole Sutron message.
///
/// # Examples
///
/// You can read packets from the filesystem:
///
/// ```
/// use sutron::{Packet, packet::Type};
/// let packet = Packet::from_path("fixtures/self-timed.sbd").unwrap();
/// assert_eq!(Type::SelfTimed, packet.type_());
/// assert!(packet.message().is_some());
/// ```
///
/// Or you can construct them yourself:
///
/// ```
/// # use sutron::{Packet, packet::Type};
/// let packet = Packet::new(b"0test message").unwrap();
/// assert_eq!(Type::SelfTimed, packet.type_());
/// assert_eq!(b"test message", packet.data());
/// assert_eq!(None, packet.message());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    data: Vec<u8>,
    message: Option<Message>,
    sub_header: Option<SubHeader>,
    station_name: Option<String>,
    type_: Type,
}

/// The type of packet.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    SelfTimed,
    EnteringAlarm,
    ExitingAlarm,
    CommandResponse,
    ForcedTransmission,
    Reserved(u8),
    UserDefined,
    BinaryData,
}

/// An error returned when trying to parse the packet.
#[derive(Debug, Fail, PartialEq)]
pub enum Error {
    /// There are characters before the first comma in the sub-header.
    #[fail(display = "unexpected leading sub-header characters: {}", _0)]
    LeadingSubHeaderCharacters(String),

    /// There is no packet id in the sub-header.
    #[fail(display = "missing id in the sub-header")]
    MissingId,

    /// There is no packet type byte to start the packet.
    #[fail(display = "there is no packet type byte")]
    MissingPacketTypeByte,

    /// There is no start byte field in the sub-header.
    #[fail(display = "missing start byte in the sub-header")]
    MissingStartByte,

    /// The station name is incorrectly specfied.
    #[fail(display = "the station name field is incorrectly specified")]
    InvalidStationNameField(String),

    /// There are characters after the sub-header that shouldn't be there.
    #[fail(display = "trailing sub-header characters: {}", _0)]
    TrailingSubHeaderCharacters(String),
}

/// A packet sub-header.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubHeader {
    /// The identifier of the packet.
    pub id: u8,

    /// The byte number at which this packet starts.
    pub start_byte: usize,

    /// The total bytes in the complete message.
    pub total_bytes: Option<usize>,
}

impl Packet {
    /// Reads a packet from an SBD message on the filesystem.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// let packet = Packet::from_path("fixtures/self-timed.sbd").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Packet, FailureError> {
        Packet::from_message(Message::from_path(path)?)
    }

    /// Creates a packet from an `sbd::mo::Message`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate sbd;
    /// # extern crate sutron;
    /// # fn main() {
    /// use sbd::mo::Message;
    /// use sutron::Packet;
    ///
    /// let message = Message::from_path("fixtures/self-timed.sbd").unwrap();
    /// let packet = Packet::from_message(message.clone()).unwrap();
    /// assert_eq!(Some(&message), packet.message());
    /// # }
    /// ```
    pub fn from_message(message: Message) -> Result<Packet, FailureError> {
        let mut packet = Packet::new(message.payload())?;
        packet.message = Some(message);
        Ok(packet)
    }

    /// Creates a new packet from a slice of u8s.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// let packet = Packet::new(b"0self timed message").unwrap();
    /// ```
    pub fn new(bytes: &[u8]) -> Result<Packet, FailureError> {
        let mut iter = bytes.iter().map(|&n| n);
        let mut type_byte;
        loop {
            type_byte = iter.next().ok_or(Error::MissingPacketTypeByte)?;
            if type_byte != LOOK_TO_NEXT_BYTE_FOR_MEANING {
                break;
            }
        }
        let mut sub_header: Option<SubHeader> = None;
        let mut station_name: Option<String> = None;
        if is_extended(type_byte) {
            let s = String::from_utf8(
                iter.by_ref()
                    .take_while(|&n| n != SUB_HEADER_TERMINATOR)
                    .collect(),
            )?;
            let mut fields = s.split(',');
            if let Some(first) = fields.next() {
                if first != "" {
                    return Err(Error::LeadingSubHeaderCharacters(first.to_string()).into());
                }
            }
            let id: u8 = fields
                .next()
                .ok_or(FailureError::from(Error::MissingId))
                .and_then(|s| s.parse().map_err(FailureError::from))?;
            let start_byte: usize = fields
                .next()
                .ok_or(FailureError::from(Error::MissingStartByte))
                .and_then(|s| s.parse().map_err(FailureError::from))?;
            let mut total_bytes: Option<usize> = None;
            if let Some(next) = fields.next() {
                if let Some(s) = parse_sub_header_station_name(next) {
                    station_name = Some(s);
                } else {
                    total_bytes = Some(next.parse()?);
                }
            }
            if let Some(next) = fields.next() {
                station_name = Some(
                    parse_sub_header_station_name(next)
                        .ok_or(Error::InvalidStationNameField(next.to_string()))?,
                );
            }
            let remaining_characters = fields.collect::<Vec<&str>>().join(",");
            if remaining_characters.len() > 0 {
                return Err(Error::TrailingSubHeaderCharacters(format!(
                    ",{}",
                    remaining_characters
                )).into());
            }
            sub_header = Some(SubHeader {
                id: id,
                start_byte: start_byte,
                total_bytes: total_bytes,
            })
        } else {
            let breadcrumb_iter = iter.clone();
            if let Some(name) = station_name_from_non_extended_sub_header(&mut iter) {
                station_name = Some(name);
            } else {
                iter = breadcrumb_iter;
            }
        }
        Ok(Packet {
            data: iter.collect(),
            message: None,
            station_name: station_name,
            sub_header: sub_header,
            type_: Type::from(type_byte),
        })
    }

    /// Returns true if this packet starts a new Sutron message.
    ///
    /// True for non-extended packets or extended packets with a `total_bytes` field.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// assert!(Packet::new(b"0test").unwrap().is_start_packet());
    /// assert!(Packet::new(b"1,42,0,4:test").unwrap().is_start_packet());
    /// assert!(!Packet::new(b"1,42,2:test").unwrap().is_start_packet());
    /// ```
    pub fn is_start_packet(&self) -> bool {
        self.sub_header
            .map(|h| h.total_bytes.is_some())
            .unwrap_or(true)
    }

    /// Returns this packet's type.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::{Packet, packet::Type};
    /// let packet = Packet::from_path("fixtures/self-timed.sbd").unwrap();
    /// assert_eq!(Type::SelfTimed, packet.type_());
    /// let packet = Packet::from_path("fixtures/forced.sbd").unwrap();
    /// assert_eq!(Type::ForcedTransmission, packet.type_());
    /// ```
    pub fn type_(&self) -> Type {
        self.type_
    }

    /// Returns this packet's sub-header.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// let packet = Packet::new(b"0test").unwrap();
    /// assert_eq!(None, packet.sub_header());
    /// let packet = Packet::new(b"1,42,22:test").unwrap();
    /// let sub_header = packet.sub_header().unwrap();
    /// assert_eq!(42, sub_header.id);
    /// assert_eq!(22, sub_header.start_byte);
    /// assert_eq!(None, sub_header.total_bytes);
    /// ```
    pub fn sub_header(&self) -> Option<SubHeader> {
        self.sub_header
    }

    /// Return this packet's station name.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// let packet = Packet::from_path("fixtures/self-timed.sbd").unwrap();
    /// assert!(packet.station_name().is_none());
    /// let packet = Packet::from_path("fixtures/forced.sbd").unwrap();
    /// assert_eq!("ATLAS_02", packet.station_name().unwrap());
    /// ```
    pub fn station_name(&self) -> Option<&str> {
        self.station_name.as_ref().map(|s| s.as_str())
    }

    /// Returns a reference to this packet's data.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// let packet = Packet::from_path("fixtures/forced.sbd").unwrap();
    /// assert_eq!(b"test message from pete at ATLAS North 2018-07-30 1235".as_ref(), packet.data());
    /// ```
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns a reference to this packet's message.
    ///
    /// Only populated if this packet was created from an SBD message.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// let packet = Packet::from_path("fixtures/self-timed.sbd").unwrap();
    /// assert!(packet.message().is_some());
    /// let packet = Packet::new(b"0self-timed message").unwrap();
    /// assert!(packet.message().is_none());
    /// ```
    pub fn message(&self) -> Option<&Message> {
        self.message.as_ref()
    }

    /// Returns this packet's datetime, as derived from its message's time of session.
    ///
    /// # Examples
    ///
    /// ```
    /// use sutron::Packet;
    /// let packet = Packet::from_path("fixtures/self-timed.sbd").unwrap();
    /// assert!(packet.datetime().is_some());
    /// ```
    pub fn datetime(&self) -> Option<DateTime<Utc>> {
        self.message.as_ref().map(|m| m.time_of_session())
    }
}

impl From<u8> for Type {
    fn from(n: u8) -> Type {
        match n {
            b'0' | b'1' => Type::SelfTimed,
            b'2' | b'3' => Type::EnteringAlarm,
            b'4' | b'5' => Type::ExitingAlarm,
            b'6' | b'7' => Type::CommandResponse,
            b'8' | b'9' => Type::ForcedTransmission,
            b'}' => Type::UserDefined,
            0xff => Type::BinaryData,
            _ => Type::Reserved(n),
        }
    }
}

fn is_extended(n: u8) -> bool {
    (n > b'0') & (n <= b'9') & (n % 2 == 1)
}

fn parse_sub_header_station_name(s: &str) -> Option<String> {
    if s.starts_with("N=") {
        Some(s[2..].to_string())
    } else {
        None
    }
}

fn station_name_from_non_extended_sub_header<I>(iter: I) -> Option<String>
where
    I: Iterator<Item = u8>,
{
    let sub_header = String::from_utf8(iter.take_while(|&n| n != SUB_HEADER_TERMINATOR).collect())
        .unwrap_or(String::new());
    if sub_header.starts_with(',') {
        parse_sub_header_station_name(&sub_header[1..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_file() {
        let packet = Packet::from_path("fixtures/self-timed-extended-0.sbd").unwrap();
        assert_eq!(Type::SelfTimed, packet.type_());
        assert_eq!(None, packet.station_name());
        let sub_header = packet.sub_header().unwrap();
        assert_eq!(26, sub_header.id);
        assert_eq!(0, sub_header.start_byte);
        assert_eq!(Some(433), sub_header.total_bytes);

        let packet = Packet::from_path("fixtures/forced.sbd").unwrap();
        assert_eq!(
            b"test message from pete at ATLAS North 2018-07-30 1235".as_ref(),
            packet.data()
        );
    }

    #[test]
    fn new() {
        assert_eq!(
            Error::MissingPacketTypeByte,
            Packet::new(b"").unwrap_err().downcast().unwrap()
        );
        assert_eq!(
            Error::LeadingSubHeaderCharacters("2".to_string()),
            Packet::new(b"12,:").unwrap_err().downcast().unwrap()
        );
        assert_eq!(
            Error::MissingId,
            Packet::new(b"1:").unwrap_err().downcast().unwrap()
        );
        assert_eq!(
            Error::MissingStartByte,
            Packet::new(b"1,42:").unwrap_err().downcast().unwrap()
        );

        let packet = Packet::new(b"1,42,16:").unwrap();
        let sub_header = packet.sub_header.unwrap();
        assert_eq!(42, sub_header.id);
        assert_eq!(16, sub_header.start_byte);
        assert_eq!(None, sub_header.total_bytes);

        let packet = Packet::new(b"1,42,16,22:").unwrap();
        let sub_header = packet.sub_header.unwrap();
        assert_eq!(42, sub_header.id);
        assert_eq!(16, sub_header.start_byte);
        assert_eq!(Some(22), sub_header.total_bytes);

        let packet = Packet::new(b"1,42,16,N=ATLAS:").unwrap();
        assert_eq!(Some("ATLAS"), packet.station_name());
        let sub_header = packet.sub_header.unwrap();
        assert_eq!(42, sub_header.id);
        assert_eq!(16, sub_header.start_byte);
        assert_eq!(None, sub_header.total_bytes);

        let packet = Packet::new(b"1,42,16,22,N=ATLAS:").unwrap();
        assert_eq!(Some("ATLAS"), packet.station_name());
        let sub_header = packet.sub_header.unwrap();
        assert_eq!(42, sub_header.id);
        assert_eq!(16, sub_header.start_byte);
        assert_eq!(Some(22), sub_header.total_bytes);

        assert_eq!(
            Error::InvalidStationNameField("ATLAS".to_string()),
            Packet::new(b"1,42,16,22,ATLAS:")
                .unwrap_err()
                .downcast()
                .unwrap()
        );
        assert_eq!(
            Error::TrailingSubHeaderCharacters(",foobar".to_string()),
            Packet::new(b"1,42,16,22,N=ATLAS,foobar:")
                .unwrap_err()
                .downcast()
                .unwrap()
        );

        let packet = Packet::new(b"0").unwrap();
        assert_eq!(None, packet.station_name());
        let packet = Packet::new(b"0,N=ATLAS:beers").unwrap();
        assert_eq!(Some("ATLAS"), packet.station_name());
        assert_eq!(b"beers", packet.data());
        let packet = Packet::new(b"0,ATLAS:").unwrap();
        assert_eq!(None, packet.station_name());
        assert_eq!(b",ATLAS:", packet.data());
    }

    #[test]
    fn is_extended() {
        for &n in &[b'1', b'3', b'5', b'7', b'9'] {
            assert!(super::is_extended(n));
        }
        for &n in &[b'0', b'2', b'4', b'6', b'8', b'}', b'!', 0xff] {
            assert!(!super::is_extended(n));
        }
    }

    #[test]
    fn parse_sub_header_station_name() {
        assert_eq!(None, super::parse_sub_header_station_name("ATLAS"));
        assert_eq!(
            Some("ATLAS".to_string()),
            super::parse_sub_header_station_name("N=ATLAS")
        );
    }
}
