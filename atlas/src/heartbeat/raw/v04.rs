//! Version 04 of the heartbeats was installed in September 2018.

use super::v03::{BAD, COULD_NOT_OPEN};
use byteorder::ReadBytesExt;
use std::io::Read;

/// Information about the efoys.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Efoys(pub [Option<Efoy>; 2]);

/// Information about one efoy.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Efoy {
    /// The same information that was transmitted in version 03.
    pub efoy: super::v03::Efoy,

    /// The active cartridge port.
    pub active_cartridge_port: u8,
}

impl Efoys {
    /// Reads EFOY data from a read.
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::heartbeat::raw::v04::{Efoys, Efoy};
    /// use std::io::Cursor;
    ///
    /// let cursor = Cursor::new(b"bb");
    /// assert_eq!(Efoys([None, None]), Efoys::read_from(cursor).unwrap());
    ///
    /// let mut bytes = vec![b'g'];
    /// bytes.extend([0; 24].iter());
    /// bytes.push(b'b');
    /// let cursor = Cursor::new(bytes);
    /// assert_eq!(Efoys([Some(Efoy::default()), None]), Efoys::read_from(cursor).unwrap());
    /// ```
    pub fn read_from<R: Read>(mut read: R) -> Result<Efoys, ::failure::Error> {
        let mut efoys = [None, None];
        for efoy in &mut efoys {
            let byte = read.read_u8()?;
            *efoy = if byte == COULD_NOT_OPEN || byte == BAD {
                None
            } else {
                Some(Efoy {
                    efoy: super::v03::Efoy::read_from(&mut read)?,
                    active_cartridge_port: read.read_u8()?,
                })
            };
        }
        Ok(Efoys(efoys))
    }
}

#[cfg(test)]
mod tests {
    use heartbeat::raw::Heartbeat;

    #[test]
    fn fixtures() {
        Heartbeat::new(include_bytes!("../../../fixtures/03/atlas-north.hb")).unwrap();
        Heartbeat::new(include_bytes!("../../../fixtures/03/atlas-south.hb")).unwrap();
    }
}
