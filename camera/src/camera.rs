//! The `Camera` struct and helper types.

use chrono::Duration;
use failure::Error;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use Image;

/// A remote camera.
#[derive(Clone, Debug)]
pub struct Camera {
    path: PathBuf,
}

/// An error returned when there's a problem calculating a camera's interval.
#[derive(Debug, Fail)]
pub enum IntervalError {
    /// There are no images in this camera.
    NoImages(Camera),

    /// There are two or more intervals tied for the most common.
    Ambiguous(BTreeSet<Duration>),
}

impl Camera {
    /// Creates a bunch of cameras from a root path and returns them as a map.
    ///
    /// # Examples
    ///
    /// ```
    /// use camera::Camera;
    /// let cameras = Camera::from_root_path("fixtures/camera/from_root_path/one").unwrap();
    /// assert_eq!(1, cameras.len());
    /// assert!(cameras.contains_key("camera"));
    /// ```
    pub fn from_root_path<P: AsRef<Path>>(root_path: P) -> Result<BTreeMap<String, Camera>, Error> {
        use walkdir::WalkDir;

        let mut cameras = BTreeMap::new();
        for entry in WalkDir::new(&root_path).min_depth(1) {
            let entry = entry?;
            let path = entry.path();
            let camera = Camera::from_path(path);
            if camera
                .images()
                .map(|images| !images.is_empty())
                .unwrap_or(false)
            {
                cameras.insert(
                    path.strip_prefix(&root_path)?
                        .to_string_lossy()
                        .into_owned(),
                    camera,
                );
            }
        }
        Ok(cameras)
    }

    /// Creates a camera from a fileystem path.
    ///
    /// # Examples
    ///
    /// ```
    /// use camera::Camera;
    /// let camera = Camera::from_path(".");
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Camera {
        Camera {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Returns a vector of this camera's images.
    ///
    /// Returns an error if this camera's path does not exist or is not a directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use camera::{Camera, Image};
    /// let camera = Camera::from_path("fixtures/camera/images/one");
    /// let images = camera.images().unwrap();
    /// ```
    pub fn images(&self) -> io::Result<Vec<Image>> {
        let mut images = self
            .path
            .read_dir()?
            .filter_map(|result| {
                result
                    .ok()
                    .and_then(|entry| Image::from_path(entry.path()).ok())
            })
            .collect::<Vec<Image>>();
        images.sort();
        Ok(images)
    }

    /// Returns this camera's interval, as determined by its images.
    ///
    /// # Examples
    ///
    /// ```
    /// use camera::Camera;
    /// let camera = Camera::from_path("fixtures/camera/interval/three_hours");
    /// assert_eq!(3, camera.interval().unwrap().num_hours());
    /// ```
    pub fn interval(&self) -> Result<Duration, Error> {
        let images = self.images()?;
        let mut durations = BTreeMap::new();
        for (a, b) in images.iter().zip(images.iter().skip(1)) {
            let duration = b.datetime() - a.datetime();
            let count = durations.entry(duration).or_insert(0);
            *count += 1;
        }
        if durations.is_empty() {
            return Err(IntervalError::NoImages(self.clone()).into());
        }
        let max_count = durations.values().max().unwrap();
        let durations = durations
            .iter()
            .filter_map(|(&duration, count)| {
                if count == max_count {
                    Some(duration)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        if durations.len() == 1 {
            Ok(durations.into_iter().next().unwrap())
        } else {
            Err(IntervalError::Ambiguous(durations).into())
        }
    }
}

impl fmt::Display for IntervalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IntervalError::NoImages(ref camera) => {
                write!(f, "no images for camera at path: {}", camera.path.display())
            }
            IntervalError::Ambiguous(ref durations) => {
                let durations = durations
                    .iter()
                    .map(|duration| format!("{} sec", duration.num_seconds()))
                    .collect::<Vec<_>>();
                write!(
                    f,
                    "more than one duration was the most common: {}",
                    durations.join(", ")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod images {
        use super::*;

        #[test]
        fn empty() {
            let camera = Camera::from_path("fixtures/camera/images/empty");
            assert_eq!(0, camera.images().unwrap().len());
        }

        #[test]
        fn one() {
            let camera = Camera::from_path("fixtures/camera/images/one");
            assert_eq!(1, camera.images().unwrap().len());
        }

        #[test]
        fn non_image() {
            let camera = Camera::from_path("fixtures/camera/images/non_image");
            assert_eq!(0, camera.images().unwrap().len());
        }
    }

    mod from_root_path {
        use super::*;

        #[test]
        fn none() {
            let cameras = Camera::from_root_path("fixtures/camera/from_root_path/none").unwrap();
            assert_eq!(0, cameras.len());
        }

        #[test]
        fn empty() {
            let cameras = Camera::from_root_path("fixtures/camera/from_root_path/empty").unwrap();
            assert_eq!(0, cameras.len());
        }

        #[test]
        fn one() {
            let cameras = Camera::from_root_path("fixtures/camera/from_root_path/one").unwrap();
            assert_eq!(1, cameras.len());
            assert!(cameras.contains_key("camera"));
        }

        #[test]
        fn dual() {
            let cameras = Camera::from_root_path("fixtures/camera/from_root_path/dual").unwrap();
            assert_eq!(2, cameras.len());
            assert!(cameras.contains_key("camera/dual1"));
            assert!(cameras.contains_key("camera/dual2"));
        }

        #[test]
        fn one_and_dual() {
            let cameras =
                Camera::from_root_path("fixtures/camera/from_root_path/one_and_dual").unwrap();
            assert_eq!(3, cameras.len());
            assert!(cameras.contains_key("camera"));
            assert!(cameras.contains_key("camera/dual1"));
            assert!(cameras.contains_key("camera/dual2"));
        }
    }

    mod interval {
        use super::*;

        #[test]
        fn no_images() {
            let camera = Camera::from_path("fixtures/camera/interval/no_images");
            assert!(camera.interval().is_err());
        }

        #[test]
        fn one_image() {
            let camera = Camera::from_path("fixtures/camera/interval/one_image");
            assert!(camera.interval().is_err());
        }

        #[test]
        fn three_hours() {
            let camera = Camera::from_path("fixtures/camera/interval/three_hours");
            assert_eq!(Duration::hours(3), camera.interval().unwrap());
        }

        #[test]
        fn ambiguous() {
            let camera = Camera::from_path("fixtures/camera/interval/ambiguous");
            assert!(camera.interval().is_err());
        }

        #[test]
        fn three_hours_with_gaps() {
            let camera = Camera::from_path("fixtures/camera/interval/three_hours_with_gaps");
            assert_eq!(Duration::hours(3), camera.interval().unwrap());
        }
    }
}
