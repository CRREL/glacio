extern crate actix_web;
#[macro_use]
extern crate clap;
// TODO put this behind a feature gate.
extern crate listenfd;
extern crate web;

use actix_web::server;
use clap::App;
use web::Config;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let addr = matches.value_of("ADDR").unwrap();
    let config = Config::from_path(matches.value_of("CONFIG").unwrap())
        .expect("could not read configuration");
    let auto_reload = matches.is_present("auto-reload");

    if auto_reload {
        use listenfd::ListenFd;
        let mut listenfd = ListenFd::from_env();
        let mut server = server::new(move || web::create_app(config.clone()));
        server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
            server.listen(l)
        } else {
            server.bind(addr).unwrap()
        };
        server.run();
    } else {
        server::new(move || web::create_app(config.clone()))
            .bind(addr)
            .unwrap()
            .run()
    }
}
