//! Raw heartbeat information.
//!
//! These structures map more-or-less directly onto the bytes that are received from the remote
//! systems.
//!
//! # Examples
//!
//! Create a new heartbeat from a bunch of bytes:
//!
//! ```
//! use atlas::heartbeat::raw::Heartbeat;
//! let heartbeat = Heartbeat::new(include_bytes!("../../fixtures/03/atlas-north.hb")).unwrap();
//! ```

use std::io::{Cursor, Read};

const MAGIC_NUMBER: [u8; 4] = *b"ATHB";

/// An ATLAS heartbeat.
#[derive(Clone, Debug, Serialize)]
pub enum Heartbeat {
    /// Version 03 of heartbeat messages began in July 2018 and ended in September 2018.
    Version03 {
        /// Information about the batteries at the site.
        batteries: version_03::Batteries,

        /// Information about the EFOYs at the site.
        efoys: version_03::Efoys,

        /// Information from the weather sensors.
        sensors: version_03::Sensors,

        /// Information from the wind sensor.
        ///
        /// The north site has one, the south site doesn't.
        wind: Option<version_03::Wind>,

        /// Information about the scanner logs.
        scanner: version_03::Scanner,
    },

    /// Version 04 of heartbeat messages began in September 2018.
    ///
    /// It's identical to version 03, except that the efoys have one extra byte (the active
    /// sensor).
    Version04 {
        /// Information about the batteries at the site.
        batteries: version_03::Batteries,

        /// Information about the EFOYs at the site.
        efoys: version_04::Efoys,

        /// Information from the weather sensors.
        sensors: version_03::Sensors,

        /// Information from the wind sensor.
        ///
        /// The north site has one, the south site doesn't.
        wind: Option<version_03::Wind>,

        /// Information about the scanner logs.
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

/// Version 03 of the heartbeats was in commission from 2018-07 through 2018-09.
pub mod version_03 {
    use byteorder::{LittleEndian, ReadBytesExt};
    use heartbeat;
    use std::io::{Cursor, Read};

    /// A connection to the device could not be opened.
    pub const COULD_NOT_OPEN: u8 = b'x';

    /// The device responded well.
    pub const GOOD: u8 = b'g';

    /// The device responded poorly.
    pub const BAD: u8 = b'b';

    /// Four K2 batteries were installed at each site.
    ///
    /// If the can232 adapter, which is used to communicate with the four batteries, couldn't be opened,
    /// then this will be None. If an individual K2 didn't respond, its entry will be none.
    #[derive(Clone, Debug, Serialize)]
    pub struct Batteries(pub Option<[Option<K2>; 4]>);

    /// K2 battery information.
    #[derive(Clone, Debug, Serialize)]
    pub struct K2 {
        /// The battery voltage [V].
        pub voltage: f32,

        /// The battery current [A].
        ///
        /// Negative is flowing into the battery.
        pub current: f32,

        /// The battery temperature [C].
        pub temperature: i8,

        /// The state of charge of the battery [%].
        pub state_of_charge: u8,

        /// The battery status byte.
        pub status: u8,

        /// The shutdown codes.
        pub shutdown_codes: u16,

        /// The error codes.
        pub error_codes: u16,

        /// The warning codes.
        pub warning_codes: u16,

        /// Additional information about the battery.
        pub additional_information: u8,
    }

    /// Two EFOYs were installed at each site.
    #[derive(Clone, Debug, Serialize)]
    pub struct Efoys(pub [Option<Efoy>; 2]);

    /// EFOY status information.
    #[derive(Clone, Debug, Serialize)]
    pub struct Efoy {
        /// The internal temperature of the EFOY [C].
        pub internal_temperature: f32,

        /// The battery voltage seen by the efoy [V].
        pub battery_voltage: f32,

        /// The output current of the EFOY [A].
        pub output_current: f32,

        /// The reservoir fluid level [%].
        pub reservoir_fluid_level: f32,

        /// The current error byte.
        pub current_error: u8,

        /// The amount of methanol consumed [L].
        pub methanol_consumption: f32,

        /// The operating mode of the EFOY.
        pub mode: u8,

        /// The status of the EFOY.
        pub status: u8,
    }

    /// Information from the weather sensors.
    #[derive(Clone, Debug, Serialize)]
    pub struct Sensors {
        /// The barometric pressure inside of the power box [mbar].
        pub barometric_pressure: f32,

        /// The temperature, as measured by the barometric pressure sensor inside of the power box
        /// [C].
        pub power_box_temperature: f32,

        /// The external temperature [C].
        pub external_temperature: f32,

        /// The relative humidity [%].
        pub relative_humidity: f32,
    }

    /// Wind sensor data.
    #[derive(Clone, Debug, Serialize)]
    pub struct Wind {
        /// The wind speed [m/s, maybe?].
        pub speed: f32,

        /// The wind direction [deg].
        pub direction: f32,
    }

    /// Scanner log data.
    #[derive(Clone, Debug, Serialize)]
    pub struct Scanner {
        /// The string of information logged when the scanner powered on.
        pub power_on: String,

        /// Logged when the scan starts.
        pub start_scan: String,

        /// Logged when the scan stops.
        pub stop_scan: String,

        /// Logged when the scan is skipped.
        pub skip_scan: String,
    }

    impl Batteries {
        /// Reads battery information from a cursor.
        pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Batteries, ::failure::Error> {
            let position = cursor.position();
            if cursor.read_u8()? == COULD_NOT_OPEN {
                Ok(Batteries(None))
            } else {
                cursor.set_position(position);
                let mut batteries = [None, None, None, None];
                for mut battery in &mut batteries {
                    match cursor.read_u8()? {
                        GOOD => *battery = Some(K2::read_from(cursor)?),
                        BAD => *battery = None,
                        n @ _ => return Err(super::Error::UnexpectedByte(n).into()),
                    }
                }
                Ok(Batteries(Some(batteries)))
            }
        }
    }

    impl From<Batteries> for Vec<heartbeat::Battery> {
        fn from(batteries: Batteries) -> Vec<heartbeat::Battery> {
            batteries
                .0
                .map(|array| {
                    array
                        .into_iter()
                        .filter_map(|o| o.clone().map(|b| b.into()))
                        .collect()
                })
                .unwrap_or_else(Vec::new)
        }
    }

    impl K2 {
        fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<K2, ::failure::Error> {
            Ok(K2 {
                voltage: cursor.read_f32::<LittleEndian>()?,
                current: cursor.read_f32::<LittleEndian>()?,
                temperature: cursor.read_i8()?,
                state_of_charge: cursor.read_u8()?,
                status: cursor.read_u8()?,
                shutdown_codes: cursor.read_u16::<LittleEndian>()?,
                error_codes: cursor.read_u16::<LittleEndian>()?,
                warning_codes: cursor.read_u16::<LittleEndian>()?,
                additional_information: cursor.read_u8()?,
            })
        }
    }

    impl From<K2> for heartbeat::Battery {
        fn from(battery: K2) -> heartbeat::Battery {
            heartbeat::Battery {
                state_of_charge: battery.state_of_charge.into(),
            }
        }
    }

    impl Efoys {
        /// Reads the efoys from a cursor.
        pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Efoys, ::failure::Error> {
            let mut efoys = [None, None];
            for efoy in &mut efoys {
                let byte = cursor.read_u8()?;
                *efoy = if byte == COULD_NOT_OPEN || byte == BAD {
                    None
                } else {
                    Some(Efoy::read_from(cursor)?)
                };
            }
            Ok(Efoys(efoys))
        }
    }

    impl Efoy {
        /// Reads an efoy from a cursor.
        pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Efoy, ::failure::Error> {
            Ok(Efoy {
                internal_temperature: cursor.read_f32::<LittleEndian>()?,
                battery_voltage: cursor.read_f32::<LittleEndian>()?,
                output_current: cursor.read_f32::<LittleEndian>()?,
                reservoir_fluid_level: cursor.read_f32::<LittleEndian>()?,
                current_error: cursor.read_u8()?,
                methanol_consumption: cursor.read_f32::<LittleEndian>()?,
                mode: cursor.read_u8()?,
                status: cursor.read_u8()?,
            })
        }
    }

    impl Sensors {
        /// Reads the sensor data from the cursor.
        pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Sensors, ::failure::Error> {
            Ok(Sensors {
                barometric_pressure: cursor.read_f32::<LittleEndian>()?,
                power_box_temperature: cursor.read_f32::<LittleEndian>()?,
                external_temperature: cursor.read_f32::<LittleEndian>()?,
                relative_humidity: cursor.read_f32::<LittleEndian>()?,
            })
        }
    }

    impl Wind {
        /// Reads the wind data from the cursor.
        pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Wind, ::failure::Error> {
            Ok(Wind {
                speed: cursor.read_f32::<LittleEndian>()?,
                direction: cursor.read_f32::<LittleEndian>()?,
            })
        }
    }

    impl From<Wind> for heartbeat::Wind {
        fn from(wind: Wind) -> heartbeat::Wind {
            heartbeat::Wind {
                speed: wind.speed,
                direction: wind.direction,
            }
        }
    }

    impl Scanner {
        /// Reads the scanner data from the cursor.
        pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Scanner, ::failure::Error> {
            use regex::Regex;

            let mut string = String::new();
            cursor.read_to_string(&mut string)?;
            lazy_static! {
                static ref RE: Regex = Regex::new("^power_on=(?P<power_on>.*),start_scan=(?P<start_scan>.*),stop_scan=(?P<stop_scan>.*),skip_scan=(?P<skip_scan>.*)$").unwrap();
            }
            if let Some(captures) = RE.captures(&string) {
                Ok(Scanner {
                    power_on: captures.name("power_on").unwrap().as_str().to_string(),
                    start_scan: captures.name("start_scan").unwrap().as_str().to_string(),
                    stop_scan: captures.name("stop_scan").unwrap().as_str().to_string(),
                    skip_scan: captures.name("skip_scan").unwrap().as_str().to_string(),
                })
            } else {
                Err(super::Error::RegexMismatch(string.clone()).into())
            }
        }
    }
}

/// Version 04 of the heartbeats was installed in September 2018.
pub mod version_04 {
    use super::version_03::{BAD, COULD_NOT_OPEN};
    use byteorder::ReadBytesExt;
    use std::io::Cursor;

    /// Information about the efoys.
    #[derive(Clone, Debug, Serialize)]
    pub struct Efoys([Option<Efoy>; 2]);

    /// Information about one efoy.
    #[derive(Clone, Debug, Serialize)]
    pub struct Efoy {
        /// The same information that was transmitted in version 03.
        pub efoy: super::version_03::Efoy,

        /// The active cartridge port.
        pub active_cartridge_port: u8,
    }

    impl Efoys {
        /// Reads efoys from a cursor.
        pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Efoys, ::failure::Error> {
            let mut efoys = [None, None];
            for efoy in &mut efoys {
                let byte = cursor.read_u8()?;
                *efoy = if byte == COULD_NOT_OPEN || byte == BAD {
                    None
                } else {
                    Some(Efoy {
                        efoy: super::version_03::Efoy::read_from(cursor)?,
                        active_cartridge_port: cursor.read_u8()?,
                    })
                };
            }
            Ok(Efoys(efoys))
        }
    }
}

impl Heartbeat {
    /// Creates a new heartbeat from bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::heartbeat::raw::Heartbeat;
    /// let heartbeat = Heartbeat::new(include_bytes!("../../fixtures/03/atlas-north.hb")).unwrap();
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
        Heartbeat::new(include_bytes!("../../fixtures/03/atlas-north.hb")).unwrap();
        Heartbeat::new(include_bytes!("../../fixtures/03/atlas-south.hb")).unwrap();
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
