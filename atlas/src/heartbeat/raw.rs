//! Raw heartbeat information.
//!
//! These structures map more-or-less directly onto the bytes that are received from the remote
//! systems.

/// Version 3 of heartbeats was deployed in July 2018.
pub mod v3 {
    use byteorder::{LittleEndian, ReadBytesExt};
    use failure::Error as FailureError;
    use regex::Regex;
    use std::io::Read;

    const NEEDLE: &[u8] = b"power_on=";
    const ATLAS_SOUTH_NEEDLE_POSITION: usize = 149;
    const ATLAS_NORTH_NEEDLE_POSITION: usize = 157;
    const ATLAS_SOUTH_HAS_WIND_SENSOR: bool = false;
    const ATLAS_NORTH_HAS_WIND_SENSOR: bool = true;
    const COULD_NOT_OPEN: u8 = b'x';
    const GOOD: u8 = b'g';
    const BAD: u8 = b'b';

    /// A version 3 heartbeat for ATLAS, begun transmitting 2018-07.
    ///
    /// One quirk is that the north has a wind sensor and the south doesn't.
    #[derive(Debug, Default, Serialize)]
    #[allow(missing_docs)]
    pub struct Heartbeat {
        pub header: [u8; 6],
        pub total_bytes: [u8; 3],
        pub batteries: Option<[Option<Battery>; 4]>,
        pub efoys: [Option<Efoy>; 2],
        pub barometric_pressure: f32,
        pub temperature_from_barometer: f32,
        pub temperature: f32,
        pub relative_humidity: f32,
        pub wind: Option<Wind>,
        pub power_on: String,
        pub start_scan: String,
        pub stop_scan: String,
        pub skip_scan: String,
    }

    impl Heartbeat {
        /// Creates a new heartbeat from some bytes.
        ///
        /// The second argument is true if the heartbeat has wind sensor data, false if not.
        ///
        /// # Examples
        ///
        /// ```
        /// use atlas::heartbeat::raw::v3::Heartbeat;
        /// let heartbeat = Heartbeat::new(include_bytes!("../../fixtures/atlas-north.hb")).unwrap();
        /// ```
        pub fn new(bytes: &[u8]) -> Result<Heartbeat, FailureError> {
            use byteorder::{LittleEndian, ReadBytesExt};
            use std::io::{Cursor, Read};

            let has_wind_sensor = if let Some(position) = bytes
                .windows(NEEDLE.len())
                .position(|window| window == NEEDLE)
            {
                if position == ATLAS_NORTH_NEEDLE_POSITION {
                    ATLAS_NORTH_HAS_WIND_SENSOR
                } else if position == ATLAS_SOUTH_NEEDLE_POSITION {
                    ATLAS_SOUTH_HAS_WIND_SENSOR
                } else {
                    return Err(Error::HasWindSensorHeuristicFail.into());
                }
            } else {
                return Err(Error::MissingPattern(NEEDLE.to_vec()).into());
            };

            let mut heartbeat = Heartbeat::default();
            let mut cursor = Cursor::new(bytes);
            cursor.read_exact(&mut heartbeat.header)?;
            cursor.read_exact(&mut heartbeat.total_bytes)?;
            let position = cursor.position();
            if cursor.read_u8()? != COULD_NOT_OPEN {
                cursor.set_position(position);
                heartbeat.batteries = Some(Default::default());
                for battery in heartbeat.batteries.as_mut().unwrap().iter_mut() {
                    *battery = Battery::maybe_read_from(&mut cursor)?;
                }
            }
            for efoy in heartbeat.efoys.iter_mut() {
                *efoy = Efoy::maybe_read_from(&mut cursor)?;
            }
            heartbeat.barometric_pressure = cursor.read_f32::<LittleEndian>()?;
            heartbeat.temperature_from_barometer = cursor.read_f32::<LittleEndian>()?;
            heartbeat.temperature = cursor.read_f32::<LittleEndian>()?;
            heartbeat.relative_humidity = cursor.read_f32::<LittleEndian>()?;
            if has_wind_sensor {
                heartbeat.wind = Some(Wind::read_from(&mut cursor)?);
            }
            let mut remainder = String::new();
            cursor.read_to_string(&mut remainder)?;
            lazy_static! {
                static ref RE: Regex = Regex::new("^power_on=(?P<power_on>.*),start_scan=(?P<start_scan>.*),stop_scan=(?P<stop_scan>.*),skip_scan=(?P<skip_scan>.*)$").unwrap();
            }
            if let Some(captures) = RE.captures(&remainder) {
                heartbeat.power_on = captures.name("power_on").unwrap().as_str().to_string();
                heartbeat.start_scan = captures.name("start_scan").unwrap().as_str().to_string();
                heartbeat.stop_scan = captures.name("stop_scan").unwrap().as_str().to_string();
                heartbeat.skip_scan = captures.name("skip_scan").unwrap().as_str().to_string();
            }
            Ok(heartbeat)
        }
    }

    /// Wind information.
    #[derive(Debug, Default, Serialize)]
    pub struct Wind {
        /// The wind speed.
        pub speed: f32,

        /// The wind direction.
        pub direction: f32,
    }

    /// A k2 battery status heartbeat.
    #[derive(Debug, Default, Serialize)]
    #[allow(missing_docs)]
    pub struct Battery {
        pub voltage: f32,
        pub current: f32,
        pub temperature: u8,
        pub state_of_charge: u8,
        pub status: u8,
        pub shutdown_codes: u16,
        pub error_codes: u16,
        pub warning_codes: u16,
        pub additional_information: u8,
    }

    /// An EFOY status heartbeat.
    #[derive(Debug, Default, Serialize)]
    #[allow(missing_docs)]
    pub struct Efoy {
        pub internal_temperature: f32,
        pub battery_voltage: f32,
        pub output_current: f32,
        pub reservoir_fluid_level: f32,
        pub current_error: u8,
        pub methanol_consumption: f32,
        pub mode: u8,
        pub status: u8,
    }

    /// An error returned when reading a raw heartbeat message.
    #[derive(Debug, Fail)]
    pub enum Error {
        /// The heuristic to determine if the message has a wind sensor failed.
        #[fail(display = "unable to determine if the heartbeat had wind sensor data")]
        HasWindSensorHeuristicFail,

        /// We were expecting a certain pattern in the message, and we couldn't find it.
        #[fail(display = "missing pattern: {:?}", _0)]
        MissingPattern(Vec<u8>),

        /// Status bytes indicate whether a given component responded or not.
        #[fail(display = "unexpected status byte: {}", _0)]
        UnexpectedStatusByte(u8),
    }

    impl Wind {
        fn read_from<R: Read>(mut read: R) -> Result<Wind, FailureError> {
            let mut wind = Wind::default();
            wind.speed = read.read_f32::<LittleEndian>()?;
            wind.direction = read.read_f32::<LittleEndian>()?;
            Ok(wind)
        }
    }

    impl Battery {
        fn maybe_read_from<R: Read>(mut read: R) -> Result<Option<Battery>, FailureError> {
            let byte = read.read_u8()?;
            if byte == GOOD {
                Battery::read_from(read).map(|b| Some(b))
            } else if byte == BAD {
                Ok(None)
            } else {
                Err(Error::UnexpectedStatusByte(byte).into())
            }
        }

        fn read_from<R: Read>(mut read: R) -> Result<Battery, FailureError> {
            let mut battery = Battery::default();
            battery.voltage = read.read_f32::<LittleEndian>()?;
            battery.current = read.read_f32::<LittleEndian>()?;
            battery.temperature = read.read_u8()?;
            battery.state_of_charge = read.read_u8()?;
            battery.status = read.read_u8()?;
            battery.shutdown_codes = read.read_u16::<LittleEndian>()?;
            battery.error_codes = read.read_u16::<LittleEndian>()?;
            battery.warning_codes = read.read_u16::<LittleEndian>()?;
            battery.additional_information = read.read_u8()?;
            Ok(battery)
        }
    }

    impl Efoy {
        // TODO we can probably trait this out
        fn maybe_read_from<R: Read>(mut read: R) -> Result<Option<Efoy>, FailureError> {
            let byte = read.read_u8()?;
            if byte == GOOD {
                Efoy::read_from(read).map(|b| Some(b))
            } else if byte == BAD {
                Ok(None)
            } else {
                Err(Error::UnexpectedStatusByte(byte).into())
            }
        }

        fn read_from<R: Read>(mut read: R) -> Result<Efoy, FailureError> {
            let mut efoy = Efoy::default();
            efoy.internal_temperature = read.read_f32::<LittleEndian>()?;
            efoy.battery_voltage = read.read_f32::<LittleEndian>()?;
            efoy.output_current = read.read_f32::<LittleEndian>()?;
            efoy.reservoir_fluid_level = read.read_f32::<LittleEndian>()?;
            efoy.current_error = read.read_u8()?;
            efoy.methanol_consumption = read.read_f32::<LittleEndian>()?;
            efoy.mode = read.read_u8()?;
            efoy.status = read.read_u8()?;
            Ok(efoy)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures() {
        v3::Heartbeat::new(include_bytes!("../../fixtures/atlas-north.hb")).unwrap();
        v3::Heartbeat::new(include_bytes!("../../fixtures/atlas-south.hb")).unwrap();
    }
}
