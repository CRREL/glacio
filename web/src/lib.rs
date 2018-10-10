//! A JSON HTTP web API for our glacier data.

#![deny(missing_docs, missing_debug_implementations, unsafe_code)]

extern crate actix_web;
extern crate atlas;
extern crate camera;
extern crate chrono;
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;
extern crate url;

pub mod config;
pub mod handler;

pub use config::Config;

use actix_web::{middleware::cors::Cors, App};

/// Creates the web application.
///
/// # Examples
///
/// ```
/// use web::Config;
/// let config = Config::from_path("fixtures/config.toml").unwrap();
/// let app = web::create_app(config);
/// ```
pub fn create_app(config: Config) -> App<Config> {
    App::with_state(config).configure(|app| {
        Cors::for_app(app)
            .send_wildcard()
            .resource("/atlas", |resource| resource.h(handler::atlas_sites))
            .resource("/atlas/{id}", |resource| {
                resource.name("site");
                resource.h(handler::atlas_site)
            })
            .resource("/cameras", |resource| resource.h(handler::cameras))
            .resource("/cameras/{id}", |resource| {
                resource.name("camera");
                resource.h(handler::camera)
            })
            .resource("/cameras/{id}/images", |resource| {
                resource.h(handler::camera_images_default)
            })
            .resource("/cameras/{id}/images/{subcamera_id}", |resource| {
                resource.name("camera_images");
                resource.h(handler::camera_images)
            })
            .register()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::Method;
    use actix_web::test::TestServer;
    use actix_web::HttpMessage;
    use handler::{Camera, Image, Site};
    use serde::de::DeserializeOwned;
    use serde_json;
    use std::str;

    #[test]
    fn cameras() {
        let cameras: Vec<Camera> = get("/cameras");
        assert_eq!(2, cameras.len());
    }

    #[test]
    fn camera() {
        let camera: Camera = get("/cameras/ATLAS_CAM");
        assert_eq!("ATLAS_CAM", camera.id);
        assert_eq!("ATLAS context", camera.name);
        assert!(camera.url.ends_with("/cameras/ATLAS_CAM"));
    }

    #[test]
    fn camera_images() {
        let images: Vec<Image> = get("/cameras/ATLAS_CAM/images");
        assert_eq!(2, images.len());
    }

    #[test]
    fn sites() {
        let sites: Vec<Site> = get("/atlas");
        assert_eq!(2, sites.len());
    }

    #[test]
    fn site() {
        let site: Site = get("/atlas/north");
        assert_eq!("ATLAS North", site.name);
        assert_eq!("north", site.id);
        assert!(site.url.ends_with("/atlas/north"));
    }

    fn test_state() -> Config {
        Config::from_path("fixtures/config.toml").unwrap()
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
}
