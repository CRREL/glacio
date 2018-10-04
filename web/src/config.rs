//! Configuration for the web api.
//!
//! This configurations is used as a shared state for all requests, driving how the requests are
//! handled.
//!
//! # Examples
//!
//! Configs are usually specified in TOML files:
//!
//! ```
//! use web::Config;
//! let config = Config::from_path("fixtures/config.toml").unwrap();
//! ```

use atlas;
use camera;
use failure::Error;
use std::path::{Path, PathBuf};
use url::Url;

/// Configure the JSON API.
///
/// # Examples
///
/// This is generally done through deserialization of a toml file:
///
/// ```
/// use web::Config;
/// let config = Config::from_path("fixtures/config.toml").unwrap();
/// ```
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Config {
    image_document_root: PathBuf,
    image_server: String,
    cameras: Vec<Camera>,

    iridium_sbd_root: PathBuf,
    #[serde(rename = "atlas")]
    sites: Vec<Site>,
}

/// Camera configuration.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Camera {
    /// A single camera.
    ///
    /// Only one camera in the box, nice and simple.
    Single {
        /// The name of the camera, more descriptive.
        name: String,

        /// The id of the camera, shorter and more slug-like.
        id: String,

        /// A longer description of the camera.
        description: String,

        /// The directory that holds the camera's images.
        path: PathBuf,
    },

    /// A dual camera.
    ///
    /// There's two images in the box. Though the whole box has one name and id, there's two images
    /// that come off of it for each snapshot.
    Dual {
        /// The name of the camera, a more descriptive string.
        name: String,

        /// The id of the camera, a short string.
        id: String,

        /// A longer description of the camera.
        description: String,

        /// The two image directory paths.
        paths: [PathBuf; 2],
    },
}

/// ATLAS site configuration.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Site {
    /// The name of the site.
    pub name: String,

    /// The id of the site.
    pub id: String,
}

impl Config {
    /// Reads configuration from a toml file.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = web::Config::from_path("fixtures/config.toml").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        use std::fs::File;
        use std::io::Read;
        use toml;

        let mut file = File::open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        toml::from_str(&string).map_err(Error::from)
    }

    /// Returns this configuration's camera configurations.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = web::Config::from_path("fixtures/config.toml").unwrap();
    /// let cameras = config.cameras();
    /// ```
    pub fn cameras(&self) -> &[Camera] {
        &self.cameras
    }

    /// Returns a camera configuration by id, or none if one does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = web::Config::from_path("fixtures/config.toml").unwrap();
    /// let camera = config.camera("ATLAS_CAM").unwrap();
    /// assert_eq!("ATLAS_CAM", camera.id());
    /// assert_eq!(None, config.camera("NOTACAMERA"));
    /// ```
    pub fn camera(&self, id: &str) -> Option<&Camera> {
        self.cameras.iter().find(|camera| camera.id() == id)
    }

    /// Returns the image url for a camera image.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::Config;
    /// let config = Config::from_path("fixtures/config.toml").unwrap();
    /// let image = config.cameras()[0].latest_image().unwrap();
    /// let url = config.image_url(&image).unwrap();
    /// assert_eq!("http://iridiumcam.lidar.io/ATLAS_CAM/ATLAS_CAM_20180614_120000.jpg", url);
    /// ```
    pub fn image_url(&self, image: &camera::Image) -> Result<String, Error> {
        let path = image.path();
        let child = path.strip_prefix(&self.image_document_root)?;
        let url = Url::parse(&self.image_server)?.join(child.to_string_lossy().as_ref())?;
        Ok(url.into_string())
    }

    /// Returns a reference to a slice of all of the sites.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = web::Config::from_path("fixtures/config.toml").unwrap();
    /// let sites = config.sites();
    /// ```
    pub fn sites(&self) -> &[Site] {
        &self.sites
    }

    /// Returns the site with the provided id.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::Config;
    /// let config = Config::from_path("fixtures/config.toml").unwrap();
    /// let site = config.site("north").unwrap();
    /// assert_eq!(None, config.site("not a site"));
    /// ```
    pub fn site(&self, id: &str) -> Option<&Site> {
        self.sites.iter().find(|site| site.id() == id)
    }

    /// Returns the latest heartbeat from the provided site.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let config = web::Config::from_path("fixtures/config.toml").unwrap();
    /// let latest_heartbeat = config.latest_heartbeat("north").unwrap();
    /// ```
    pub fn latest_heartbeat(&self, id: &str) -> Option<atlas::Heartbeat> {
        id.parse::<atlas::Site>()
            .ok()
            .and_then(|site| site.heartbeats(&self.iridium_sbd_root).ok())
            .and_then(|mut heartbeats| heartbeats.pop())
    }
}

impl Camera {
    /// Returns this camera's id.
    ///
    /// The id is a short name, good for URLs.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::config::Camera;
    /// let camera = Camera::from("fixtures/ATLAS_CAM");
    /// assert_eq!("ATLAS_CAM", camera.id());
    /// ```
    pub fn id(&self) -> &str {
        match *self {
            Camera::Single { ref id, .. } | Camera::Dual { ref id, .. } => id.as_str(),
        }
    }

    /// Returns this camera's name.
    ///
    /// This should be more human-readable than the id.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::config::Camera;
    /// let camera = Camera::from("fixtures/ATLAS_CAM");
    /// assert_eq!("ATLAS_CAM", camera.name());
    /// ```
    pub fn name(&self) -> &str {
        match *self {
            Camera::Single { ref name, .. } | Camera::Dual { ref name, .. } => name.as_str(),
        }
    }

    /// Returns this camera's description.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::config::Camera;
    /// let camera = Camera::from("fixtures/ATLAS_CAM");
    /// assert_eq!("", camera.description());
    /// ```
    pub fn description(&self) -> &str {
        match *self {
            Camera::Single {
                ref description, ..
            }
            | Camera::Dual {
                ref description, ..
            } => description.as_str(),
        }
    }

    /// Returns the latest image for this camera.
    ///
    /// In the case of dual cameras, returns the most recent image from either camera. If there's a
    /// tie, prefers an image from the first camera.
    ///
    /// If the camera doesn't have any images in it, or doesn't point to a directory, returns None.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::config::Camera;
    /// let camera = Camera::from("fixtures/ATLAS_CAM");
    /// let image = camera.latest_image().unwrap();
    /// ```
    pub fn latest_image(&self) -> Option<camera::Image> {
        match *self {
            Camera::Single { ref path, .. } => camera::Camera::from_path(path).latest_image(),
            Camera::Dual { ref paths, .. } => {
                let mut images: Vec<_> = paths
                    .iter()
                    .filter_map(|path| camera::Camera::from_path(path).latest_image())
                    .collect();
                match images.len() {
                    0 | 1 => images.pop(),
                    2 => {
                        if images[0].datetime() == images[1].datetime() {
                            Some(images.remove(0))
                        } else {
                            images.sort();
                            images.pop()
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl<P: AsRef<Path>> From<P> for Camera {
    fn from(path: P) -> Camera {
        let file_name = path
            .as_ref()
            .file_name()
            .map(|file_name| file_name.to_string_lossy().to_string())
            .unwrap_or_else(String::new);
        Camera::Single {
            id: file_name.clone(),
            name: file_name,
            path: path.as_ref().to_path_buf(),
            description: String::new(),
        }
    }
}

impl Site {
    /// Returns this site's id.
    ///
    /// # Examples
    ///
    /// ```
    /// let site = web::config::Site::from("north");
    /// assert_eq!("north", site.id());
    /// ```
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Returns this site's name.
    ///
    /// # Examples
    ///
    /// ```
    /// let site = web::config::Site::from("north");
    /// assert_eq!("north", site.name());
    /// ```
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl<S: AsRef<str>> From<S> for Site {
    fn from(s: S) -> Site {
        Site {
            id: s.as_ref().to_string(),
            name: s.as_ref().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures() {
        Config::from_path("fixtures/config.toml").unwrap();
    }
}
