//! Raw heartbeats map more-or-less directly onto the bytes in the heartbeat messages.
//!
//! Generally downstreams shouldn't use raw heartbeats, except to pluck any information that isn't
//! included in the higher level `atlas::Heartbeat`.
//!
//! # Examples
//!
//! Create a new heartbeat from a bunch of bytes:
//!
//! ```
//! use atlas::heartbeat::raw::Heartbeat;
//! let bytes = include_bytes!("../../../fixtures/03/atlas-north.hb");
//! let heartbeat = Heartbeat::new(bytes).unwrap();
//! ```

pub mod version_03;
pub mod version_04;

use std::io::{Cursor, Read};

const MAGIC_NUMBER: [u8; 4] = *b"ATHB";

/// An enum that contains all raw versions of ATLAS heartbeats supported by this crate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Heartbeat {
    /// Version 03 of heartbeat messages began in July 2018 and ended in September 2018.
    Version03 {
        /// Each site has four K2 batteries.
        batteries: version_03::Batteries,

        /// Each site has two EFOYs.
        ///
        /// EFOYs are methanol fuel cells that should take over once the solar isn't enough to keep
        /// the system powered.
        efoys: version_03::Efoys,

        /// Both sites have an identical suite of weather sensors.
        sensors: version_03::Sensors,

        /// The north site has a wind sensor, but the south site doesn't.
        wind: Option<version_03::Wind>,

        /// The scanner saves ASCII messages to the data logger, which are trasmitted back in
        /// heartbeats as-is.
        scanner: version_03::Scanner,
    },

    /// Version 04 of heartbeat messages began in September 2018.
    ///
    /// It's identical to version 03 except that the efoy data includes one extra byte to indicate
    /// the currently active cartridge.
    Version04 {
        /// Each site has four K2 batteries.
        batteries: version_03::Batteries,

        /// Each site has two EFOYs.
        ///
        /// The EFOY data for version 04 has one extra byte, the active cartridge, from version 03.
        efoys: version_04::Efoys,

        /// Both sites has an identical suite of weather sensor.
        sensors: version_03::Sensors,

        /// The north site has a wind sensor, but the south site doesn't.
        wind: Option<version_03::Wind>,

        /// The scanner saves ASCII messages to the data logger, which are trasmitted back in
        /// heartbeats as-is.
        scanner: version_03::Scanner,
    },
}

/// An error returned when reading a raw heartbeat message.
#[derive(Debug, Fail, PartialEq)]
pub enum Error {
    /// The magic number is invalid.
    #[fail(display = "invalid magic number: {:?}", _0)]
    MagicNumber([u8; 4]),

    /// A regular expression was expected to match this string, but it didn't.
    #[fail(display = "regex did not match string: {}", _0)]
    RegexMismatch(String),

    /// The version is invalid.
    ///
    /// This might not mean that no heartbeat versions of this type exist, but just that we can't
    /// read them.
    #[fail(display = "invalid version: {}", _0)]
    Version(u8),

    /// An unexpected byte was encountered when reading raw bytes.
    #[fail(display = "unexpected byte: {}", _0)]
    UnexpectedByte(u8),
}

impl Heartbeat {
    /// Creates a new heartbeat from bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::heartbeat::raw::Heartbeat;
    /// let heartbeat = Heartbeat::new(include_bytes!("../../../fixtures/03/atlas-north.hb")).unwrap();
    /// ```
    pub fn new(bytes: &[u8]) -> Result<Heartbeat, ::failure::Error> {
        let mut cursor = Cursor::new(bytes);
        let mut magic_number = [0u8; 4];
        cursor.read_exact(&mut magic_number)?;
        if magic_number != MAGIC_NUMBER {
            return Err(Error::MagicNumber(magic_number).into());
        }
        let mut version = [0u8; 2];
        cursor.read_exact(&mut version)?;
        let version = String::from_utf8(version.to_vec())?.parse()?;
        let mut length = [0u8; 3];
        cursor.read_exact(&mut length)?;
        match version {
            3 => Heartbeat::read_version_03_from(cursor),
            4 => Heartbeat::read_version_04_from(cursor),
            _ => return Err(Error::Version(version).into()),
        }
    }

    fn read_version_03_from(mut cursor: Cursor<&[u8]>) -> Result<Heartbeat, ::failure::Error> {
        use self::version_03::*;
        let batteries = Batteries::read_from(&mut cursor)?;
        let efoys = Efoys::read_from(&mut cursor)?;
        let sensors = Sensors::read_from(&mut cursor)?;
        let mut wind = None;
        let position = cursor.position();
        let scanner = if let Ok(scanner) = Scanner::read_from(&mut cursor) {
            scanner
        } else {
            cursor.set_position(position);
            wind = Some(Wind::read_from(&mut cursor)?);
            Scanner::read_from(&mut cursor)?
        };
        Ok(Heartbeat::Version03 {
            batteries: batteries,
            efoys: efoys,
            sensors: sensors,
            wind: wind,
            scanner: scanner,
        })
    }

    fn read_version_04_from(mut cursor: Cursor<&[u8]>) -> Result<Heartbeat, ::failure::Error> {
        use self::version_03::{Batteries, Scanner, Sensors, Wind};
        use self::version_04::Efoys;

        let batteries = Batteries::read_from(&mut cursor)?;
        let efoys = Efoys::read_from(&mut cursor)?;
        let sensors = Sensors::read_from(&mut cursor)?;
        let mut wind = None;
        let position = cursor.position();
        let scanner = if let Ok(scanner) = Scanner::read_from(&mut cursor) {
            scanner
        } else {
            cursor.set_position(position);
            wind = Some(Wind::read_from(&mut cursor)?);
            Scanner::read_from(&mut cursor)?
        };
        Ok(Heartbeat::Version04 {
            batteries: batteries,
            efoys: efoys,
            sensors: sensors,
            wind: wind,
            scanner: scanner,
        })
    }
}

impl From<Heartbeat> for ::Heartbeat {
    fn from(heartbeat: Heartbeat) -> ::Heartbeat {
        match heartbeat.clone() {
            Heartbeat::Version03 {
                batteries, wind, ..
            } => ::Heartbeat {
                datetime: None,
                batteries: batteries.into(),
                wind: wind.map(|w| w.into()),
                raw: heartbeat,
            },
            Heartbeat::Version04 {
                batteries, wind, ..
            } => ::Heartbeat {
                datetime: None,
                batteries: batteries.into(),
                wind: wind.map(|w| w.into()),
                raw: heartbeat,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures() {
        Heartbeat::new(include_bytes!("../../../fixtures/04/atlas-north.hb")).unwrap();
        Heartbeat::new(include_bytes!("../../../fixtures/04/atlas-south.hb")).unwrap();
    }

    #[test]
    fn header() {
        assert_eq!(
            Error::MagicNumber(*b"PETE"),
            Heartbeat::new(b"PETE").unwrap_err().downcast().unwrap()
        );
    }

    #[test]
    fn version() {
        assert_eq!(
            Error::Version(1),
            Heartbeat::new(b"ATHB01000")
                .unwrap_err()
                .downcast()
                .unwrap()
        );
        assert_eq!(
            Error::Version(2),
            Heartbeat::new(b"ATHB02000")
                .unwrap_err()
                .downcast()
                .unwrap()
        );
        assert_eq!(
            Error::Version(5),
            Heartbeat::new(b"ATHB05000")
                .unwrap_err()
                .downcast()
                .unwrap()
        );
    }
}
