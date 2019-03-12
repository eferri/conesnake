use treesnake::board::Board;
use treesnake::config::Config;
use treesnake::log::log_init;
use treesnake::search::Node;
use treesnake::server::Server;
use treesnake::util::mem_usage;

use std::{process, sync::Arc};

use clap::Parser;
use log::info;

fn main() {
    log_init();

    let args = Config::parse();

    let start_bytes = mem_usage();
    let _test_node = Node::new(Board::new(0, 0, args.max_width, args.max_height, args.max_snakes));
    let after_bytes = mem_usage();

    let node_size = after_bytes - start_bytes;
    let space_size = node_size as i64 * args.max_boards as i64;

    info!("Args:\n{:#?}", args);

    info!("Size of Node: {}B", node_size);

    info!("Approx. size of search space: {}MiB", space_size >> 20);
    info!("Approx. size of search space: {}GiB", space_size >> 30);

    let server = Arc::new(Server::new(args));

    let server_cln = server.clone();
    ctrlc::set_handler(move || {
        info!("Ctrl-C caught, exiting...");
        server_cln.stop_server();
        server_cln.wait_done();
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    server.start_server();
    server.wait_done();
}
