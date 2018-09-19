#[macro_use]
extern crate clap;
extern crate sbd;
extern crate sutron;

use clap::App;
use sutron::{message::Reassembler, Packet};

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("reassemble") {
        let mut packets = matches
            .values_of("PATH")
            .unwrap()
            .map(|path| Packet::from_message(sbd::mo::Message::from_path(path).unwrap()).unwrap())
            .collect::<Vec<_>>();
        packets.sort_by_key(|packet| packet.datetime());
        let mut reassembler = Reassembler::new();
        loop {
            if packets.is_empty() {
                eprintln!("Not enough sbd messages.");
                break;
            } else {
                if let Some(message) = reassembler.add(packets.remove(0)) {
                    if packets.is_empty() {
                        use std::io::Write;
                        std::io::stdout().write(&message.data).unwrap();
                        break;
                    } else {
                        eprintln!("Too many sbd messages.");
                        break;
                    }
                }
            }
        }
    }
}
