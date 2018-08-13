extern crate camera;
extern crate chrono;
extern crate clap;
#[macro_use]
extern crate prettytable;

use camera::Camera;
use chrono::Utc;
use clap::{App, Arg, SubCommand};
use prettytable::{format, Table};
use std::collections::BTreeMap;

fn main() {
    // TODO support an rc file.
    let matches = App::new("glacio")
        .author("Pete Gadomski <pete@gadom.ski>")
        .subcommand(
            SubCommand::with_name("cameras").arg(
                Arg::with_name("ROOT")
                    .help("the root path of all of the camera files")
                    .required(true)
                    .index(1),
            ),
        )
        .subcommand(
            SubCommand::with_name("camera").arg(
                Arg::with_name("PATH")
                    .help("the path to the camera directory")
                    .required(true)
                    .index(1),
            ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("cameras") {
        let root = matches.value_of("ROOT").unwrap();
        cameras(Camera::from_root_path(root).unwrap());
    } else if let Some(matches) = matches.subcommand_matches("camera") {
        // TODO this should probably be a subcommand
        let path = matches.value_of("PATH").unwrap();
        let camera = Camera::from_path(path);
        for image in camera.images().unwrap() {
            println!(
                "{}, {}",
                image.path().display(),
                image.datetime().to_string()
            );
        }
    }
}

fn cameras(cameras: BTreeMap<String, Camera>) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row!["Name", "Interval", "Count", "Latest", "Active"]);

    for (name, camera) in cameras {
        let mut interval_string = "n/a".to_string();
        let mut latest_string = "n/a".to_string();
        let mut active = false;

        let images = camera.images().unwrap();
        if let Ok(interval) = camera.interval() {
            let seconds = interval.num_seconds();
            interval_string = if seconds % 3600 == 0 {
                format!("{} hours", interval.num_hours())
            } else if seconds % 60 == 0 {
                format!("{} minutes", interval.num_minutes())
            } else {
                format!("{} seconds", seconds)
            };

            if let Some(image) = images.last() {
                let datetime = image.datetime();
                latest_string = datetime.to_string();

                if Utc::now() - datetime < interval * 2 {
                    active = true;
                }
            }
        }
        table.add_row(row![
            name,
            interval_string,
            images.len(),
            latest_string,
            active
        ]);
    }
    table.printstd();
}
