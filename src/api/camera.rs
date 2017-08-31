use {Camera, Image, Result};
use api::Pagination;
use camera::Server;
use iron::Request;

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Serialize, Debug)]
pub struct Summary {
    name: String,
    description: String,
    url: String,
    images_url: String,
}

#[derive(Serialize, Debug)]
pub struct Detail;

#[derive(Serialize, Debug)]
pub struct ImageSummary {
    camera_name: String,
    datetime: String,
    url: String,
}

impl Config {
    pub fn summary(&self, request: &Request) -> Summary {
        let url = url_for!(request, "camera", "name" => self.name.clone());
        let images_url = url_for!(request, "camera_images", "name" => self.name.clone());
        Summary {
            name: self.name.clone(),
            description: self.description.clone(),
            url: url.as_ref().to_string(),
            images_url: images_url.as_ref().to_string(),
        }
    }

    pub fn detail(&self, _: &Request) -> Detail {
        Detail
    }

    pub fn images(&self, request: &mut Request, server: &Server) -> Result<Vec<ImageSummary>> {
        let pagination = Pagination::new(request)?;
        let mut images = self.camera()
            .and_then(|camera| camera.images())
            .and_then(|images| images.collect::<Result<Vec<_>>>())?;
        images.sort_by(|a, b| b.cmp(a));
        images.into_iter()
            .skip(pagination.skip())
            .take(pagination.take())
            .map(|image| self.image_summary(request, server, &image))
            .collect()
    }

    fn camera(&self) -> Result<Camera> {
        Camera::new(&self.path)
    }

    fn image_summary(&self, _: &Request, server: &Server, image: &Image) -> Result<ImageSummary> {
        Ok(ImageSummary {
               camera_name: self.name.to_string(),
               datetime: image.datetime().to_rfc3339(),
               url: server.url_for(image)?.to_string(),
           })
    }
}
