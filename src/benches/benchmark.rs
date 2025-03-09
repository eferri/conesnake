#![feature(test)]

extern crate test;

use conesnake::board::Board;
use conesnake::log::log_test_init;
use conesnake::pool::ThreadPool;
use conesnake::rand::FastRand;
use conesnake::search;
use conesnake::search::{SearchContext, ThreadContext};
use conesnake::tests::common::{release_config, test_game};

use std::env;
use std::str;
use std::sync::Arc;
use std::time::Instant;

use log::info;
use test::Bencher;

const BOARD: &str = "
    turn: 1 health: 100 health: 100 health: 100 health: 100
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - b > 0 - - - d - -
    - - - - - - - - v - -
    - - - - - - - - 1 - -
    - - - - - - - - - - -
    - - 2 - - - - - - - -
    - - ^ - - - - - - - -
    - - c - - - 3 < a - -
    - - - - - - - - - - -
    - - - - - - - - - - -";

#[bench]
fn playout_bench(b: &mut Bencher) {
    let mut cfg = release_config();

    log_test_init();

    cfg.max_boards = 1000;
    cfg.max_snakes = 4;
    cfg.max_width = 10;
    cfg.max_height = 10;

    let res = env::var("COMPARE");
    cfg.compare = str::parse::<u32>(res.unwrap().as_str()).unwrap() == 1;

    #[cfg(feature = "simd")]
    info!("using simd");

    if cfg.compare {
        info!("compare is true!")
    }

    info!("running benchmark...");

    let game = test_game();
    let start_board = Board::from_str(BOARD, &game).unwrap();

    let mut ctx = ThreadContext::<FastRand>::new(&cfg);

    b.iter(|| {
        ctx.board = start_board.clone();
        let _ = search::playout_game(&cfg, &mut ctx, &game);
    });
}

#[bench]
fn search_bench(b: &mut Bencher) {
    let mut cfg = release_config();

    log_test_init();

    cfg.max_boards = 20000;
    cfg.max_snakes = 4;
    cfg.max_width = 11;
    cfg.max_height = 11;
    cfg.num_threads = 8;
    cfg.latency = 0;
    cfg.fixed_iter = true;
    cfg.iter = 1000;

    let res = env::var("COMPARE");
    cfg.compare = str::parse::<u32>(res.unwrap().as_str()).unwrap() == 1;

    #[cfg(feature = "simd")]
    info!("using simd");

    if cfg.compare {
        info!("compare is true!")
    }

    info!("allocating...");

    let ctx = Arc::new(SearchContext::<FastRand>::new(&cfg));
    let pool = Arc::new(ThreadPool::new(ctx.config.num_threads));

    let game = Arc::new(test_game());
    let board = Arc::new(Board::from_str(BOARD, &game).unwrap());

    info!("running benchmark...");

    b.iter(|| search::mcts(ctx.clone(), &pool, &board, &game, Instant::now()).unwrap());
}
