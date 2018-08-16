extern crate actix_web;
extern crate camera;
extern crate failure;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod resource;

use actix_web::{http::Method, App};
use failure::Error;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Build the web application.
pub fn app(state: State) -> App<State> {
    App::with_state(state).resource("/cameras", |r| {
        r.method(Method::GET).f(resource::cameras::index)
    })
}

/// The shared state of the web application.
#[derive(Debug, Clone)]
pub struct State {
    /// All cameras that will be served.
    pub cameras: BTreeMap<String, camera::Camera>,
}

#[derive(Debug, Deserialize)]
struct Config {
    camera_root_path: PathBuf,
}

impl State {
    /// Creates a state from a path to a toml configuration file.
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

        let mut file = File::open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let config: Config = toml::de::from_str(&string)?;
        Ok(State {
            cameras: ::camera::Camera::from_root_path(config.camera_root_path)?,
        })
    }
}
