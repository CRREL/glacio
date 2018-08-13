//! The `Image` struct and associated helper types.

use chrono::{DateTime, TimeZone, Utc};
use regex::Regex;
use std::fmt;
use std::path::{Path, PathBuf};

/// The regular expression used to identify remote image files.
pub const IMAGE_FILE_NAME_REGEX: &str = r"^([[:word:]]+)_(?P<year>\d{4})(?P<month>\d{2})(?P<day>\d{2})_(?P<hour>\d{2})(?P<minute>\d{2})(?P<second>\d{2}).jpg$";

/// A remote camera image.
#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Ord)]
pub struct Image {
    datetime: DateTime<Utc>,
    path: PathBuf,
}

/// Error returned when trying to open an image with an invalid file name.
#[derive(Debug, Fail, PartialEq)]
pub struct InvalidFileName(PathBuf);

impl Image {
    /// Creates an image from a path on the filesystem.
    ///
    /// # Examples
    ///
    /// ```
    /// use camera::Image;
    /// let image = Image::from_path("ATLAS_CAM_20180813_182500.jpg").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Image, InvalidFileName> {
        lazy_static! {
            static ref RE: Regex = Regex::new(IMAGE_FILE_NAME_REGEX).unwrap();
        }
        let file_name = path
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if let Some(captures) = RE.captures(file_name) {
            Ok(Image {
                datetime: Utc
                    .ymd(
                        captures["year"].parse().unwrap(),
                        captures["month"].parse().unwrap(),
                        captures["day"].parse().unwrap(),
                    )
                    .and_hms(
                        captures["hour"].parse().unwrap(),
                        captures["minute"].parse().unwrap(),
                        captures["second"].parse().unwrap(),
                    ),
                path: path.as_ref().to_path_buf(),
            })
        } else {
            Err(InvalidFileName(path.as_ref().to_path_buf()))
        }
    }

    /// Returns this image's datetime.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate chrono;
    /// # extern crate camera;
    /// use chrono::{Utc, TimeZone};
    /// use camera::Image;
    /// # fn main () {
    /// let image = Image::from_path("ATLAS_CAM_20180813_182500.jpg").unwrap();
    /// assert_eq!(Utc.ymd(2018, 8, 13).and_hms(18, 25, 0), image.datetime());
    /// # }
    /// ```
    pub fn datetime(&self) -> DateTime<Utc> {
        self.datetime
    }

    /// Returns this image's path.
    ///
    /// # Examples
    ///
    /// ```
    /// use camera::Image;
    /// let image = Image::from_path("ATLAS_CAM_20180813_182500.jpg").unwrap();
    /// let path = image.path();
    /// ```
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl fmt::Display for InvalidFileName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid file name: {}", self.0.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn image_from_path() {
        let image = Image::from_path("ATLAS_CAM_20180813_182500.jpg").unwrap();
        assert_eq!(Utc.ymd(2018, 8, 13).and_hms(18, 25, 0), image.datetime());

        let image = Image::from_path("ATLAS_CAM2_StarDot1_20180813_132500.jpg").unwrap();
        assert_eq!(Utc.ymd(2018, 8, 13).and_hms(13, 25, 0), image.datetime());

        assert_eq!(
            InvalidFileName(PathBuf::from("Cargo.toml")),
            Image::from_path("Cargo.toml").unwrap_err()
        );
    }
}
