//! ATLAS heartbeats.
//!
//! There's a two-tier setup here:
//! - Messages (bytes) are parsed into "raw" heartbeats, which map more-or-less directly onto the
//! bytes.
//! - The higher-level `Heartbeat` structure aggregates various types of raw heartbeats into a
//! common interface and does some mapping from status codes to status names.

pub mod raw;

use chrono::{DateTime, TimeZone, Utc};
use failure::Error;
use sutron::Message;

const SUTRON_DATETIME_FORMAT: &str = "%m/%d/%y %H:%M:%S";

/// An ATLAS heartbeat message.
#[derive(Debug, Serialize)]
pub struct Heartbeat {
    /// The date and time (UTC) of the first heartbeat packet.
    pub datetime: Option<DateTime<Utc>>,

    /// The external temperature.
    ///
    /// TODO units.
    pub temperature: f32,

    /// The barometric pressure.
    pub barometric_pressure: f32,

    /// The wind data.
    ///
    /// South site doesn't have any wind data.
    pub wind: Option<Wind>,

    /// Scanner information.
    pub scanner: Scanner,

    /// Battery information.
    pub batteries: Vec<Option<Battery>>,

    /// EFOY information.
    pub efoys: Vec<Option<Efoy>>,
}

/// Wind data.
pub type Wind = raw::v3::Wind;

/// Scanner status information.
#[derive(Debug, Serialize)]
pub struct Scanner {
    /// The last time the scanner powered on.
    pub last_power_on: Option<DateTime<Utc>>,

    /// The last time the scanner completed a scan.
    pub last_scan_start: Option<DateTime<Utc>>,

    /// The last time the scanner completed a scan.
    pub last_scan_end: Option<DateTime<Utc>>,

    /// The last time the scanner skipped a scan.
    pub last_scan_skip: ScanSkip,
}

/// An instance of the scan being skipped.
#[derive(Debug, Serialize)]
pub struct ScanSkip {
    /// The date and time that the scan was skipped.
    pub datetime: Option<DateTime<Utc>>,

    /// The reason the scan was skipped.
    pub reason: Option<String>,
}

/// Battery status information.
#[derive(Debug, Serialize)]
pub struct Battery {
    /// The battery voltage.
    pub voltage: f32,

    /// The battery current.
    pub current: f32,

    /// The battery temperature.
    pub temperature: f32,

    /// The battery state of charge.
    pub state_of_charge: f32,

    /// The battery status.
    pub status: K2BatteryStatus,

    /// The error codes.
    pub error_codes: Vec<K2ErrorCodes>,
}

/// The status of a k2 battery.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub enum K2BatteryStatus {
    Discharge,
    Charge,
    Idle,
}

/// The various error codes available.
#[allow(missing_docs)]
#[derive(Debug, Serialize)]
pub enum K2ErrorCodes {
    CellOvervoltage,
    CellUndervoltage,
    Overtemperature,
    Undertemperature,
    OvercurrentDischarge,
    OvercurrentCharge,
    SecuritySwitch,
    CellMonitorCommError,
}

/// EFOY status information.
#[allow(missing_docs)]
#[derive(Debug, Serialize)]
pub struct Efoy {
    pub internal_temperature: f32,
    pub battery_voltage: f32,
    pub output_current: f32,
    pub reservoir_fluid_level: f32,
    pub current_error: u8,
    pub methanol_consumption: f32,
    pub mode: EfoyMode,
    pub status: EfoyStatus,
}

/// The mode of the efoy. We control the mode via commands from the Sutron.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub enum EfoyMode {
    ManualOff,
    ManualOn,
    Automatic,
    Hybrid,
    ExternalControl,
    TransportLock,
    Unknown(u8),
}

/// The status of the EFOY.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub enum EfoyStatus {
    Off,
    Standby,
    StartPhase,
    ChargingMode,
    ShutdownProcedure,
    Antifreeze,
    BatteryProtection,
    Error,
    Interruption,
    Restart,
    TransportLock,
    Unknown(u8),
}

impl Heartbeat {
    /// Creates a new heartbeat from a Sutron message.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate sutron;
    /// # extern crate atlas;
    /// # fn main() {
    /// use sutron::Message;
    /// use atlas::Heartbeat;
    /// let message = include_bytes!("../../fixtures/atlas-north.hb").to_vec();
    /// let heartbeat = Heartbeat::new(message).unwrap();
    /// # }
    /// ```
    pub fn new<M: Into<Message>>(message: M) -> Result<Heartbeat, Error> {
        let message = message.into();
        let raw_heartbeat = raw::v3::Heartbeat::new(&message.data)?;
        Ok(Heartbeat {
            datetime: message.datetime,
            temperature: raw_heartbeat.temperature,
            barometric_pressure: raw_heartbeat.barometric_pressure,
            wind: raw_heartbeat.wind.map(|w| w.into()),
            // TODO intos
            scanner: Scanner::new(
                &raw_heartbeat.power_on,
                &raw_heartbeat.start_scan,
                &raw_heartbeat.stop_scan,
                &raw_heartbeat.skip_scan,
            ),
            batteries: raw_heartbeat
                .batteries
                .map(|array| {
                    array
                        .iter()
                        .map(|option| option.as_ref().map(|b| Battery::new(b)))
                        .collect()
                })
                .unwrap_or(Vec::new()),
            efoys: raw_heartbeat
                .efoys
                .iter()
                .map(|option| option.as_ref().map(|e| Efoy::new(e)))
                .collect(),
        })
    }
}

impl Scanner {
    fn new(power_on: &str, start_scan: &str, stop_scan: &str, skip_scan: &str) -> Scanner {
        let power_on = power_on.split(',').collect::<Vec<&str>>();
        let start_scan = start_scan.split(',').collect::<Vec<&str>>();
        let stop_scan = stop_scan.split(',').collect::<Vec<&str>>();
        Scanner {
            last_power_on: power_on
                .get(0)
                .and_then(|s| Utc.datetime_from_str(s, SUTRON_DATETIME_FORMAT).ok()),
            last_scan_start: start_scan
                .get(0)
                .and_then(|s| Utc.datetime_from_str(s, SUTRON_DATETIME_FORMAT).ok()),
            last_scan_end: stop_scan
                .get(0)
                .and_then(|s| Utc.datetime_from_str(s, SUTRON_DATETIME_FORMAT).ok()),
            last_scan_skip: ScanSkip::new(skip_scan),
        }
    }
}

impl ScanSkip {
    fn new(s: &str) -> ScanSkip {
        let fields = s.split(',').collect::<Vec<&str>>();
        ScanSkip {
            datetime: fields
                .get(0)
                .and_then(|s| Utc.datetime_from_str(s, SUTRON_DATETIME_FORMAT).ok()),
            reason: fields.get(2).map(|s| s.to_string()),
        }
    }
}

impl Battery {
    fn new(raw: &raw::v3::Battery) -> Battery {
        Battery {
            current: raw.current,
            state_of_charge: raw.state_of_charge.into(),
            status: K2BatteryStatus::from(raw.status),
            temperature: offset_k2_temperature(raw.temperature),
            voltage: raw.voltage,
            error_codes: K2ErrorCodes::flags(raw.error_codes),
        }
    }
}

impl From<u8> for K2BatteryStatus {
    fn from(n: u8) -> K2BatteryStatus {
        match n {
            1 => K2BatteryStatus::Discharge,
            2 => K2BatteryStatus::Charge,
            _ => K2BatteryStatus::Idle,
        }
    }
}

impl K2ErrorCodes {
    fn flags(n: u16) -> Vec<K2ErrorCodes> {
        let mut flags = Vec::new();
        if n & 1 == 1 {
            flags.push(K2ErrorCodes::CellOvervoltage);
        }
        if n & 2 == 2 {
            flags.push(K2ErrorCodes::CellUndervoltage);
        }
        if n & 4 == 4 {
            flags.push(K2ErrorCodes::Overtemperature);
        }
        if n & 8 == 8 {
            flags.push(K2ErrorCodes::Undertemperature);
        }
        if n & 16 == 16 {
            flags.push(K2ErrorCodes::OvercurrentDischarge);
        }
        if n & 32 == 32 {
            flags.push(K2ErrorCodes::OvercurrentCharge);
        }
        if n & 0x100 == 0x100 {
            flags.push(K2ErrorCodes::SecuritySwitch);
        }
        if n & 0x200 == 0x200 {
            flags.push(K2ErrorCodes::CellMonitorCommError);
        }
        flags
    }
}

fn offset_k2_temperature(n: u8) -> f32 {
    f32::from(n) + 40.
}

impl Efoy {
    fn new(raw: &raw::v3::Efoy) -> Efoy {
        Efoy {
            battery_voltage: raw.battery_voltage,
            current_error: raw.current_error,
            internal_temperature: raw.internal_temperature,
            methanol_consumption: raw.methanol_consumption,
            mode: EfoyMode::from(raw.mode),
            output_current: raw.output_current,
            reservoir_fluid_level: raw.reservoir_fluid_level,
            status: EfoyStatus::from(raw.status),
        }
    }
}

impl From<u8> for EfoyMode {
    fn from(n: u8) -> EfoyMode {
        match n {
            0 => EfoyMode::ManualOff,
            1 => EfoyMode::ManualOn,
            2 => EfoyMode::Automatic,
            3 => EfoyMode::Hybrid,
            4 => EfoyMode::ExternalControl,
            5 => EfoyMode::TransportLock,
            _ => EfoyMode::Unknown(n),
        }
    }
}

impl From<u8> for EfoyStatus {
    fn from(n: u8) -> EfoyStatus {
        match n {
            0 => EfoyStatus::Off,
            1 => EfoyStatus::Standby,
            2 => EfoyStatus::StartPhase,
            3 => EfoyStatus::ChargingMode,
            4 => EfoyStatus::ShutdownProcedure,
            5 => EfoyStatus::Antifreeze,
            6 => EfoyStatus::BatteryProtection,
            7 => EfoyStatus::Error,
            8 => EfoyStatus::Interruption,
            9 => EfoyStatus::Restart,
            10 => EfoyStatus::TransportLock,
            _ => EfoyStatus::Unknown(n),
        }
    }
}
