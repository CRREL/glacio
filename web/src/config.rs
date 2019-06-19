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
pub struct Camera {
    /// The name of the camera, more descriptive.
    name: String,

    /// The id of the camera, shorter and more slug-like.
    id: String,

    /// A longer description of the camera.
    description: String,

    /// The directories that hold the camera's images.
    ///
    /// Single cameras only have on path, dual cameras have two.
    paths: Vec<PathBuf>,
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
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Config, ::failure::Error> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        toml::from_str(&string).map_err(::failure::Error::from)
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
    pub fn image_url(&self, image: &camera::Image) -> Result<String, ::failure::Error> {
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

    /// Returns all heartbeats from the provided site.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let config = web::Config::from_path("fixtures/config.toml").unwrap();
    /// let heartbeats = config.heartbeats("north").unwrap();
    /// ```
    pub fn heartbeats(&self, id: &str) -> Result<Vec<atlas::Heartbeat>, ::failure::Error> {
        id.parse::<atlas::Site>()
            .map_err(::failure::Error::from)
            .and_then(|site| site.heartbeats(&self.iridium_sbd_root))
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
        self.id.as_str()
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
        self.name.as_str()
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
        self.description.as_str()
    }

    /// Returns the latest image for this camera.
    ///
    /// Uses the first path in the paths array.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::config::Camera;
    /// let camera = Camera::from("fixtures/ATLAS_CAM");
    /// let image = camera.latest_image().unwrap();
    /// ```
    pub fn latest_image(&self) -> Option<camera::Image> {
        self.paths
            .get(0)
            .map(|path| camera::Camera::from_path(path))
            .and_then(|camera| camera.images().ok())
            .and_then(|mut images| images.pop())
    }

    /// The number of subcameras in this camera.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = web::Config::from_path("fixtures/config.toml").unwrap();
    /// assert_eq!(1, config.camera("ATLAS_CAM").unwrap().subcamera_count());
    /// assert_eq!(2, config.camera("DUAL_CAM").unwrap().subcamera_count());
    /// ```
    pub fn subcamera_count(&self) -> usize {
        self.paths.len()
    }

    /// Images for the specified subcamera.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::config::Camera;
    /// let camera = Camera::from("fixtures/ATLAS_CAM");
    /// let images = camera.path(0).unwrap();
    /// ```
    pub fn path(&self, subcamera_id: usize) -> Option<&Path> {
        self.paths
            .get(subcamera_id)
            .map(|path_buf| path_buf.as_path())
    }
}

impl<P: AsRef<Path>> From<P> for Camera {
    fn from(path: P) -> Camera {
        let file_name = path
            .as_ref()
            .file_name()
            .map(|file_name| file_name.to_string_lossy().to_string())
            .unwrap_or_else(String::new);
        Camera {
            id: file_name.clone(),
            name: file_name,
            paths: vec![path.as_ref().to_path_buf()],
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
