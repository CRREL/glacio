//! A JSON HTTP web API for our glacier data.

#![deny(missing_docs, missing_debug_implementations, unsafe_code)]

extern crate actix_web;
extern crate camera;
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

mod config;
mod state;

use actix_web::error::ErrorNotFound;
use actix_web::{middleware::cors::Cors, App, HttpRequest, Json, Result};
pub use config::Config;
pub use state::State;

/// Creates the web application.
///
/// # Examples
///
/// ```
/// use web::State;
/// let state = State::from_path("fixtures/config.toml").unwrap();
/// let app = web::create_app(state);
/// ```
pub fn create_app(state: State) -> App<State> {
    App::with_state(state).configure(|app| {
        Cors::for_app(app)
            .send_wildcard()
            .resource("/cameras", |resource| resource.h(cameras))
            .resource("/cameras/{id}", |resource| {
                resource.name("camera");
                resource.h(camera)
            })
            .register()
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct Camera {
    /// The camera id.
    pub id: String,

    /// The human-redable camera name.
    pub name: String,

    /// The API URL of this camera.
    pub url: String,
}

impl Camera {
    fn new<T>(camera: &state::Camera, request: &HttpRequest<T>) -> Result<Camera> {
        Ok(Camera {
            id: camera.id.clone(),
            name: camera.name.clone(),
            url: request
                .url_for("camera", &[&camera.id])?
                .as_str()
                .to_string(),
        })
    }
}

fn cameras(request: &HttpRequest<State>) -> Result<Json<Vec<Camera>>> {
    Ok(Json(
        request
            .state()
            .cameras()
            .iter()
            .map(|camera| Camera::new(camera, request))
            .collect::<Result<Vec<_>>>()?,
    ))
}

fn camera(request: &HttpRequest<State>) -> Result<Json<Camera>> {
    let id: String = request.match_info().query("id")?;
    request
        .state()
        .camera(&id)
        .ok_or(ErrorNotFound("no camera with that id"))
        .and_then(|camera| Ok(Json(Camera::new(camera, request)?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::Method;
    use actix_web::test::TestServer;
    use actix_web::HttpMessage;
    use serde::de::DeserializeOwned;
    use serde_json;
    use std::str;

    fn test_state() -> State {
        State::from_path("fixtures/config.toml").unwrap()
    }

    fn test_server() -> TestServer {
        TestServer::with_factory(|| {
            let state = test_state();
            create_app(state)
        })
    }

    fn get<T>(path: &str) -> T
    where
        T: DeserializeOwned,
    {
        let mut server = test_server();
        let request = server.client(Method::GET, path).finish().unwrap();
        let response = server.execute(request.send()).unwrap();
        assert!(response.status().is_success());
        let bytes = server.execute(response.body()).unwrap();
        let body = str::from_utf8(&bytes).unwrap();
        serde_json::from_str(body).unwrap()
    }

    #[test]
    fn cameras() {
        let cameras: Vec<Camera> = get("/cameras");
        assert_eq!(1, cameras.len());
        let camera = &cameras[0];
        assert_eq!("ATLAS_CAM", camera.id);
        assert_eq!("ATLAS context", camera.name);
        assert!(camera.url.ends_with("/cameras/ATLAS_CAM"));
    }

    #[test]
    fn camera() {
        let camera: Camera = get("/cameras/ATLAS_CAM");
        assert_eq!("ATLAS_CAM", camera.id);
        assert_eq!("ATLAS context", camera.name);
        assert!(camera.url.ends_with("/cameras/ATLAS_CAM"));
    }
}
