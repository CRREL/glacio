extern crate clap;
extern crate sbd;
extern crate sutron;

use clap::{App, Arg};
use sbd::storage::Storage;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use sutron::{message::Reassembler, Packet};

fn main() {
    let matches = App::new("sutron")
        .about("re-assembles SBD messages that have been split by Sutron")
        .arg(
            Arg::with_name("ROOT")
                .help("the root directory of the sbd messages")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("the output directory that will contain the re-assembled messages")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("imei")
                .short("i")
                .multiple(true)
                .takes_value(true)
                .help("the IMEI numbers to re-assemble"),
        )
        .get_matches();
    let root = matches.value_of("ROOT").unwrap();
    let output = matches.value_of("OUTPUT").unwrap();
    let imeis = matches.values_of("imei").unwrap().collect::<Vec<&str>>();

    let storage = sbd::storage::FilesystemStorage::open(root).unwrap();
    for imei in imeis {
        let mut directory = Path::new(output).to_path_buf();
        directory.push(imei);
        std::fs::create_dir(&directory).unwrap();
        let mut reassembler = Reassembler::new();
        for message in storage.messages_from_imei(imei).unwrap() {
            let packet = Packet::from_message(message).unwrap();
            if let Some(message) = reassembler.add(packet) {
                let mut path = directory.clone();
                path.push(format!(
                    "{}.hb",
                    message.datetime.unwrap().format("%y%m%d_%H%M%S")
                ));
                let mut file = File::create(path).unwrap();
                file.write_all(&message.data).unwrap();
            }
        }
    }
}
