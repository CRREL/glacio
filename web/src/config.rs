/// Overall JSON API configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// The cameras to be served.
    pub cameras: Vec<Camera>,
}

/// Camera configuration.
#[derive(Debug, Deserialize)]
pub struct Camera {
    /// The name of the camera.
    pub name: String,

    /// The id of the camera.
    ///
    /// The id will be used in the URL.
    pub id: String,
}
