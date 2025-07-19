use conesnake::board::Board;
use conesnake::config::Config;
use conesnake::log::log_init;
use conesnake::pool::ThreadPool;
use conesnake::rand::FastRand;
use conesnake::search;
use conesnake::tests::common::test_game;

use clap::Parser;
use log::info;

use std::sync::Arc;
use std::time::Instant;

const BOARD_STR: &str = "
    turn: 272 health: 41 health: 97
    > v - - - - - - - - -
    ^ v - - - - - - - - -
    ^ v - - - - - - - - -
    ^ v > > v - - - - - -
    ^ v ^ - 1 - - - - - +
    ^ > ^ + - 0 - - - - -
    ^ v < - - ^ < - - - -
    ^ v ^ a - - ^ < - - -
    ^ > v - - - - ^ < - -
    ^ < v - - - - - c - -
    - ^ < - - - - - + - -
";

fn main() {
    log_init();

    let mut cfg = Config::parse();
    cfg.max_boards = 800000;

    info!("Args:\n{cfg:#?}");

    #[cfg(feature = "simd")]
    info!("using simd");

    let game = test_game();
    let board = Board::from_str(BOARD_STR, &game).unwrap();
    let ctx = Arc::new(search::SearchContext::<FastRand>::new(&cfg));
    let pool = ThreadPool::new(cfg.num_threads);

    info!("done allocating");

    let start_time = Instant::now();
    search::mcts(ctx, &pool, &board, &game, start_time).unwrap();
}
