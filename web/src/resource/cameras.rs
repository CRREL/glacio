use actix_web::{HttpRequest, Json, Result};
use State;

pub fn index(request: &HttpRequest<State>) -> Result<Json<Vec<Camera>>> {
    Ok(Json(
        request
            .state()
            .cameras
            .iter()
            .map(|(name, camera)| Camera::new(name, camera))
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct Camera {
    /// The name of the camera.
    pub name: String,
}

impl Camera {
    fn new(name: &str, _camera: &::camera::Camera) -> Camera {
        Camera {
            name: name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO test actual request plumbing.

    #[test]
    fn camera_new() {
        let source_camera = ::camera::Camera::from_path(".");
        let camera = Camera::new("a camera", &source_camera);
        assert_eq!("a camera", camera.name);
    }
}
