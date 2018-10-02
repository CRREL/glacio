//! ATLAS heartbeats, which contain status information.
//!
//! Reading heartbeats is a two-step process:
//!
//! - Raw bytes are parsed into a `raw::Heartbeat`, which maps onto the raw heartbeat data.
//! - A `raw::Heartbeat` is then turned into a `Heartbeat`, which regularizes some of the data and
//! maps status codes onto enums.

pub mod raw;

use chrono::{DateTime, Utc};
use failure::Error;
use sutron::Message;

/// An ATLAS heartbeat.
///
/// Any version of raw heartbeat can be turneed into this structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct Heartbeat {
    /// The date and time of the reception of the first heartbeat packet.
    ///
    /// If this information was not provided in the source message, it will be None.
    pub datetime: Option<DateTime<Utc>>,

    /// Battery information.
    pub batteries: Vec<Battery>,

    /// Wind information.
    pub wind: Option<Wind>,

    /// The source data.
    pub raw: raw::Heartbeat,
}

/// Battery information.
#[derive(Debug, Serialize, Deserialize)]
pub struct Battery {
    /// The current in or out of the battery [A].
    ///
    /// Current out is positive, current in is negative.
    pub current: f32,

    /// The battery state of charge [%].
    pub state_of_charge: f32,

    /// The battery temperature [C].
    pub temperature: f32,

    /// The battery voltage [V].
    pub voltage: f32,
}

/// Wind information.
#[derive(Debug, Serialize, Deserialize)]
pub struct Wind {
    /// The wind speed, in meters per second.
    pub speed: f32,

    /// The wind direction, in degrees.
    pub direction: f32,
}

impl Heartbeat {
    /// Creates a heartbeat from a Sutron message.
    ///
    /// Note that a vector of u8s can be turned into a Sutron message.
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::Heartbeat;
    /// let bytes = include_bytes!("../../fixtures/03/atlas-north.hb");
    /// let heartbeat = Heartbeat::new(&bytes.to_vec().into());
    /// ```
    pub fn new(message: &Message) -> Result<Heartbeat, Error> {
        let raw = raw::Heartbeat::new(&message.data)?;
        let mut heartbeat = Heartbeat::from(raw);
        heartbeat.datetime = message.datetime;
        Ok(heartbeat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures() {
        use chrono::TimeZone;

        let north = Heartbeat::new(
            &include_bytes!("../../fixtures/03/atlas-north.hb")
                .to_vec()
                .into(),
        ).unwrap();
        assert_eq!(None, north.datetime);

        let mut message =
            Message::from(include_bytes!("../../fixtures/03/atlas-north.hb").to_vec());
        message.datetime = Some(Utc.ymd(2019, 9, 29).and_hms(12, 1, 42));
        let north = Heartbeat::new(&message).unwrap();
        assert_eq!(message.datetime, north.datetime);

        let batteries = north.batteries;
        assert_eq!(4, batteries.len());
    }
}
