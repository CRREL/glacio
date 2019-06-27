use sbd::storage::{FilesystemStorage, Storage};
use std::path::Path;
use std::str::FromStr;
use Heartbeat;

const IMEI_SOUTH: &str = "300234063554840";
const IMEI_NORTH: &str = "300234063554810";
const IMEI_CRREL: &str = "300234063554800";

/// An ATLAS installation.
#[derive(Debug, PartialEq)]
pub enum Site {
    /// ATLAS-South, installed in 2015.
    South,

    /// ATLAS-North, installed in 2018.
    North,

    /// ATLAS-CRREL, the test system back home.
    Crrel,
}

/// A site error.
#[derive(Debug, Fail)]
pub enum Error {
    /// Invalid site name.
    #[fail(display = "invalid site name: {}", _0)]
    SiteName(String),
}

impl Site {
    /// Returns a vector of this site's heartbeats inside the provided sbd root directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use atlas::Site;
    /// let heartbeats = Site::North.heartbeats("/var/iridium").unwrap();
    /// ```
    pub fn heartbeats<P: AsRef<Path>>(&self, path: P) -> Result<Vec<Heartbeat>, ::failure::Error> {
        Ok(self
            .messages(path)?
            .into_iter()
            .filter_map(|message| Heartbeat::new(&message).ok())
            .collect())
    }

    /// Returns a vector of this site's bad heartbeats inside the provided sbd root directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use atlas::Site;
    /// let heartbeats = Site::North.bad_heartbeats("/var/iridium").unwrap();
    /// ```
    pub fn bad_heartbeats<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<::failure::Error>, ::failure::Error> {
        Ok(self
            .messages(path)?
            .into_iter()
            .filter_map(|message| Heartbeat::new(&message).err())
            .collect())
    }

    /// Returns a vector of all the reassmbled messages for this site.
    ///
    /// One message can be split up over multiple SBD messages if they're long.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use atlas::Site;
    /// let messages = Site::North.messages("/var/iridium").unwrap();
    /// ```
    pub fn messages<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<::sutron::Message>, ::failure::Error> {
        let storage = FilesystemStorage::open(path)?;
        Ok(reassemble(storage.messages_from_imei(self.imei())?)?)
    }

    /// Returns the latest heartbeat.
    ///
    /// If there are any errors or there are no heartbeats, returns None.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use atlas::Site;
    /// let heartbeat = Site::North.latest_heartbeat("/var/iridium").unwrap();
    /// ```
    pub fn latest_heartbeat<P: AsRef<Path>>(&self, path: P) -> Option<Heartbeat> {
        self.heartbeats(path).ok().and_then(|mut h| h.pop())
    }

    /// Returns this site's active IMEI.
    ///
    /// # Examples
    ///
    /// ```
    /// use atlas::Site;
    /// assert_eq!("300234063554810", Site::North.imei());
    /// assert_eq!("300234063554840", Site::South.imei());
    /// assert_eq!("300234063554800", Site::Crrel.imei());
    /// ```
    pub fn imei(&self) -> &str {
        match *self {
            Site::South => IMEI_SOUTH,
            Site::North => IMEI_NORTH,
            Site::Crrel => IMEI_CRREL,
        }
    }
}

impl FromStr for Site {
    type Err = Error;
    fn from_str(s: &str) -> Result<Site, Error> {
        match s.to_lowercase().as_str() {
            "south" => Ok(Site::South),
            "north" => Ok(Site::North),
            "crrel" => Ok(Site::Crrel),
            _ => Err(Error::SiteName(s.to_string())),
        }
    }
}

fn reassemble(
    mut sbd_messages: Vec<::sbd::mo::Message>,
) -> Result<Vec<::sutron::Message>, ::failure::Error> {
    use sutron::message::Reassembler;
    use sutron::Packet;

    sbd_messages.sort_by_key(|m| m.time_of_session());
    let mut reassembler = Reassembler::new();
    let mut messages = Vec::new();
    for sbd_message in sbd_messages {
        let packet = Packet::from_message(sbd_message)?;
        if let Some(message) = reassembler.add(packet) {
            messages.push(message);
        }
    }
    messages.sort_by_key(|m| m.datetime);
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(Site::South, "south".parse().unwrap());
        assert_eq!(Site::South, "South".parse().unwrap());
        assert_eq!(Site::South, "SOUTH".parse().unwrap());
        assert_eq!(Site::North, "north".parse().unwrap());
        assert_eq!(Site::North, "North".parse().unwrap());
        assert_eq!(Site::North, "NORTH".parse().unwrap());
        assert_eq!(Site::Crrel, "crrel".parse().unwrap());
        assert_eq!(Site::Crrel, "Crrel".parse().unwrap());
        assert_eq!(Site::Crrel, "CRREL".parse().unwrap());
        assert!("notasite".parse::<Site>().is_err());
    }
}
