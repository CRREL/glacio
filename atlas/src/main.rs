extern crate atlas;
#[macro_use]
extern crate clap;
extern crate serde;
extern crate serde_json;
extern crate sutron;

use atlas::Site;
use clap::App;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("heartbeat") {
        let site: Site = matches.value_of("SITE").unwrap().parse().unwrap();
        if let Some(heartbeat) = site
            .heartbeats(matches.value_of("ROOT").unwrap())
            .unwrap()
            .into_iter()
            .last()
        {
            println!("{}", serde_json::to_string_pretty(&heartbeat).unwrap());
        } else {
            panic!("No heartbeats available");
        }
    }

    if let Some(matches) = matches.subcommand_matches("bad-heartbeat") {
        let site: Site = matches.value_of("SITE").unwrap().parse().unwrap();
        if let Some(error) = site
            .bad_heartbeats(matches.value_of("ROOT").unwrap())
            .unwrap()
            .into_iter()
            .last()
        {
            println!("{}", error);
            println!("{}", error.backtrace());
        } else {
            panic!("No bad heartbeats available");
        }
    }

    if let Some(matches) = matches.subcommand_matches("heartbeats") {
        let site: Site = matches.value_of("SITE").unwrap().parse().unwrap();
        let heartbeats = site.heartbeats(matches.value_of("ROOT").unwrap()).unwrap();
        println!("{}", serde_json::to_string(&heartbeats).unwrap());
    }
}
