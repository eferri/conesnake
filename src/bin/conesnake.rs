use conesnake::config::Config;
use conesnake::log::log_init;
use conesnake::server::Server;

use clap::Parser;
use log::info;

#[tokio::main]
async fn main() {
    log_init();

    let args = Config::parse();

    info!("Args:\n{args:#?}");

    let server = Server::new(args);
    server.run().await;
}
