extern crate camera;
extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate prettytable;

use camera::Camera;
use chrono::Utc;
use clap::App;
use prettytable::{format, Table};

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let root = matches.value_of("ROOT").unwrap();
    let cameras = Camera::from_root_path(root).unwrap();
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
