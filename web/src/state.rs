use failure::Error;
use std::path::Path;
use {config, Config};

/// The global state for the web api.
#[derive(Clone, Debug)]
pub struct State {
    cameras: Vec<Camera>,
}

#[derive(Clone, Debug)]
pub struct Camera {
    /// The name of the camera.
    pub name: String,

    /// The ID of the camera.
    pub id: String,
}

impl State {
    /// Creates a state from the path to a TOML configuration file.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::State;
    /// let state = State::from_path("fixtures/config.toml").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<State, Error> {
        use std::fs::File;
        use std::io::Read;
        use toml;

        let mut file = File::open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let config = toml::from_str(&string)?;
        Ok(State::new(config))
    }

    fn new(config: Config) -> State {
        State {
            cameras: config.cameras.into_iter().map(Camera::new).collect(),
        }
    }

    /// Returns a slice to this state's cameras.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::State;
    /// let state = State::from_path("fixtures/config.toml").unwrap();
    /// let cameras = state.cameras();
    /// ```
    pub fn cameras(&self) -> &[Camera] {
        &self.cameras
    }

    /// Returns the camera specified by the given id, or `None` if none is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use web::State;
    /// let state = State::from_path("fixtures/config.toml").unwrap();
    /// assert!(state.camera("ATLAS_CAM").is_some());
    /// assert!(state.camera("Not a camera").is_none());
    /// ```
    pub fn camera(&self, id: &str) -> Option<&Camera> {
        self.cameras.iter().find(|camera| camera.id == id)
    }
}

impl Camera {
    fn new(config: config::Camera) -> Camera {
        Camera {
            name: config.name,
            id: config.id,
        }
    }
}
