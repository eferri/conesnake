use conesnake::config::Config;
use conesnake::log::log_init;
use conesnake::server::Server;

use std::sync::Arc;

use clap::Parser;
use log::info;

fn main() {
    log_init();

    let args = Config::parse();

    info!("Args:\n{:#?}", args);

    let server = Arc::new(Server::new(args));

    let server_cln = server.clone();
    ctrlc::set_handler(move || {
        info!("Ctrl-C caught, exiting...");
        server_cln.stop_server();
    })
    .expect("Error setting Ctrl-C handler");

    server.start_server();
    server.wait_done();
}
