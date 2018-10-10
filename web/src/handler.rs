//! Handle HTTP requests.

use actix_web::{error::ErrorNotFound, Error, HttpRequest, Json, Result};
use atlas::Heartbeat;
use camera;
use chrono::{DateTime, Utc};
use {config, Config};

/// Returns a list of all cameras.
pub fn cameras(request: &HttpRequest<Config>) -> Result<Json<Vec<Camera>>> {
    Ok(Json(
        request
            .state()
            .cameras()
            .iter()
            .map(|camera| Camera::new(camera, request))
            .collect::<Result<Vec<_>>>()?,
    ))
}

/// Looks up a camera by id.
pub fn camera(request: &HttpRequest<Config>) -> Result<Json<Camera>> {
    let id: String = request.match_info().query("id")?;
    request
        .state()
        .camera(&id)
        .ok_or(ErrorNotFound("no camera with that id"))
        .and_then(|camera| Ok(Json(Camera::new(camera, request)?)))
}

/// Returns all images for this camera and subcamera.
pub fn camera_images(request: &HttpRequest<Config>) -> Result<Json<Vec<Image>>> {
    let subcamera_id: usize = request.match_info().query("subcamera_id")?;
    camera_images_for_subcamera(subcamera_id, request)
}

/// Returns all images for this camera and the default subcamera.
pub fn camera_images_default(request: &HttpRequest<Config>) -> Result<Json<Vec<Image>>> {
    camera_images_for_subcamera(0, request)
}

/// Returns a list of all ATLAS sites.
pub fn atlas_sites(request: &HttpRequest<Config>) -> Result<Json<Vec<Site>>> {
    Ok(Json(
        request
            .state()
            .sites()
            .iter()
            .map(|site| Site::new(site, request))
            .collect::<Result<Vec<_>>>()?,
    ))
}

/// Looks up an ATLAS site by id.
pub fn atlas_site(request: &HttpRequest<Config>) -> Result<Json<Site>> {
    let id: String = request.match_info().query("id")?;
    request
        .state()
        .site(&id)
        .ok_or(ErrorNotFound("no site with that id"))
        .and_then(|site| Ok(Json(Site::new(&site, request)?)))
}

/// An ATLAS site.
#[derive(Debug, Deserialize, Serialize)]
pub struct Site {
    /// The short id of the site.
    pub id: String,

    /// The longer, more readable name for the site.
    pub name: String,

    /// The API url for this site.
    pub url: String,

    /// The most recent heartbeat received from this site.
    pub latest_heartbeat: Option<Heartbeat>,
}

/// A remote camera.
#[derive(Debug, Deserialize, Serialize)]
pub struct Camera {
    /// The short id for this camera.
    pub id: String,

    /// The longer, readable name of this camera.
    pub name: String,

    /// A description of this camera.
    pub description: String,

    /// The API url for this camera.
    pub url: String,

    /// The latest image taken by this camera.
    pub latest_image: Option<Image>,

    /// The number of subcameras in this camera.
    ///
    /// Single cameras have one subcamera, dual cameras have two.
    pub subcamera_count: usize,
}

/// An image taken by a remote camera.
#[derive(Debug, Deserialize, Serialize)]
pub struct Image {
    /// The UTC datetime that this image was taken.
    pub datetime: DateTime<Utc>,

    /// The img src url.
    pub url: String,
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
            subcamera_count: camera.subcamera_count(),
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

fn camera_images_for_subcamera(
    subcamera_id: usize,
    request: &HttpRequest<Config>,
) -> Result<Json<Vec<Image>>> {
    let id: String = request.match_info().query("id")?;
    request
        .state()
        .camera(&id)
        .ok_or(ErrorNotFound("no camera with that id"))
        .and_then(|camera| {
            camera
                .path(subcamera_id)
                .ok_or(ErrorNotFound("no subcamera with that id"))
        })
        .and_then(|path| {
            camera::Camera::from_path(path)
                .images()
                .map_err(Error::from)
        })
        .map(|images| {
            Json(
                images
                    .into_iter()
                    .filter_map(|image| Image::new(&image, request).ok())
                    .collect(),
            )
        })
}
