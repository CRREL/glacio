//! Work with remote camera images on a filesystem.
//!
//! We maintain a slew of remote cameras, mainly in Alaska and Greenland. These cameras transmit
//! messages via FTP back to home servers, and the images are then placed on the lidar.io
//! filesystem under `/home/iridiumcam`. All images under that location are available via
//! <http://iridiumcam.lidar.io>.
//!
//! # Usage
//!
//! Use `Camera::from_root_path` to auto-discover cameras in a directory tree:
//!
//! ```
//! use camera::Camera;
//! let cameras = Camera::from_root_path("fixtures/camera/from_root_path/dual").unwrap();
//! assert_eq!(2, cameras.len());
//! assert!(cameras.contains_key("camera/dual1"));
//! assert!(cameras.contains_key("camera/dual2"));
//! ```
//!
//! A `Camera` can produce a `Vec<Image>` for all images in its directory tree:
//!
//! ```
//! use camera::Camera;
//! let camera = Camera::from_path("fixtures/camera/images/one");
//! let images = camera.images().unwrap();
//! assert_eq!(1, images.len());
//! ```
//!
//! An `Image` must have a file name that ends in `%Y%m%d_%H%M%S.jpg`:
//!
//! ```
//! use camera::Image;
//! let image = Image::from_path("camera_20180614_120000.jpg").unwrap();
//! assert!(Image::from_path("not-correct.jpg").is_err());
//! ```

#![deny(missing_docs, missing_debug_implementations, unsafe_code)]

extern crate chrono;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate walkdir;

pub mod camera;
pub mod image;

pub use camera::Camera;
pub use image::Image;
