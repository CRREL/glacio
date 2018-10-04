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
pub use config::Config;

use actix_web::error::ErrorNotFound;
use actix_web::{middleware::cors::Cors, App, HttpRequest, Json, Result};
use chrono::{DateTime, Utc};

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
            .resource("/atlas", |resource| resource.h(sites))
            .resource("/atlas/{id}", |resource| {
                resource.name("site");
                resource.h(site)
            })
            .resource("/cameras", |resource| resource.h(cameras))
            .resource("/cameras/{id}", |resource| {
                resource.name("camera");
                resource.h(camera)
            })
            .register()
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct Site {
    id: String,
    name: String,
    url: String,
    latest_heartbeat: Option<atlas::Heartbeat>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Camera {
    id: String,
    name: String,
    description: String,
    url: String,
    latest_image: Option<Image>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Image {
    datetime: DateTime<Utc>,
    url: String,
}

impl Camera {
    fn new(camera: &config::Camera, request: &HttpRequest<Config>) -> Result<Camera> {
        Ok(Camera {
            id: camera.id().to_string(),
            name: camera.name().to_string(),
            description: camera.description().to_string(),
            url: request
                .url_for("camera", &[camera.id()])?
                .as_str()
                .to_string(),
            latest_image: camera
                .latest_image()
                .map(|i| Image::new(&i, request))
                .map_or(Ok(None), |r| r.map(|i| Some(i)))?,
        })
    }
}

impl Image {
    fn new(image: &camera::Image, request: &HttpRequest<Config>) -> Result<Image> {
        Ok(Image {
            datetime: image.datetime(),
            url: request.state().image_url(image)?,
        })
    }
}

impl Site {
    fn new(site: &config::Site, request: &HttpRequest<Config>) -> Result<Site> {
        Ok(Site {
            id: site.id().to_string(),
            name: site.name().to_string(),
            url: request.url_for("site", &[site.id()])?.as_str().to_string(),
            latest_heartbeat: request.state().latest_heartbeat(site.id()),
        })
    }
}

fn cameras(request: &HttpRequest<Config>) -> Result<Json<Vec<Camera>>> {
    Ok(Json(
        request
            .state()
            .cameras()
            .iter()
            .map(|camera| Camera::new(camera, request))
            .collect::<Result<Vec<_>>>()?,
    ))
}

fn camera(request: &HttpRequest<Config>) -> Result<Json<Camera>> {
    let id: String = request.match_info().query("id")?;
    request
        .state()
        .camera(&id)
        .ok_or(ErrorNotFound("no camera with that id"))
        .and_then(|camera| Ok(Json(Camera::new(camera, request)?)))
}

fn sites(request: &HttpRequest<Config>) -> Result<Json<Vec<Site>>> {
    Ok(Json(
        request
            .state()
            .sites()
            .iter()
            .map(|site| Site::new(site, request))
            .collect::<Result<Vec<_>>>()?,
    ))
}

fn site(request: &HttpRequest<Config>) -> Result<Json<Site>> {
    let id: String = request.match_info().query("id")?;
    request
        .state()
        .site(&id)
        .ok_or(ErrorNotFound("no site with that id"))
        .and_then(|site| Ok(Json(Site::new(&site, request)?)))
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
