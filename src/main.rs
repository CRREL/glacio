extern crate actix_web;
extern crate camera;
extern crate chrono;
extern crate clap;
extern crate listenfd;
#[macro_use]
extern crate prettytable;
extern crate web;

use camera::Camera;
use chrono::Utc;
use clap::{App, Arg, SubCommand};
use prettytable::{format, Table};
use std::collections::BTreeMap;
use std::net::ToSocketAddrs;

fn main() {
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
            SubCommand::with_name("serve")
                .arg(
                    Arg::with_name("ADDR")
                        .help("the address from which to serve the json api")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("CONFIG")
                        .help("the path to the configuration toml file")
                        .required(true)
                        .index(2),
                )
                .arg(
                    Arg::with_name("auto-reload")
                        .long("auto-reload")
                        .help("enable the auto-reloading development server"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("cameras") {
        let root = matches.value_of("ROOT").unwrap();
        cameras(Camera::from_root_path(root).unwrap());
    } else if let Some(matches) = matches.subcommand_matches("serve") {
        let addr = matches.value_of("ADDR").unwrap();
        let state = web::State::from_path(matches.value_of("CONFIG").unwrap()).unwrap();
        let auto_reload = matches.is_present("auto-reload");
        serve(addr, state, auto_reload);
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

fn serve<S: ToSocketAddrs>(addr: S, state: web::State, auto_reload: bool) {
    if auto_reload {
        use listenfd::ListenFd;
        let mut listenfd = ListenFd::from_env();
        let mut server = actix_web::server::new(move || web::app(state.clone()));
        server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
            server.listen(l)
        } else {
            server.bind(addr).unwrap()
        };
        server.run();
    } else {
        actix_web::server::new(move || web::app(state.clone()))
            .bind(addr)
            .unwrap()
            .run()
    }
}
