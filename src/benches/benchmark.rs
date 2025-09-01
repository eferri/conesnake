#![feature(test)]

extern crate test;

use conesnake::board::Board;
use conesnake::config::{Config, MAX_BOARD_SIZE};
use conesnake::log::log_test_init;
use conesnake::pool::ThreadPool;
use conesnake::rand::FastRand;
use conesnake::search;
use conesnake::search::{SearchContext, ThreadContext};
use conesnake::tests::common::{release_config, test_game};

use std::env;
use std::mem;
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

fn print_info(cfg: &Config) {
    if cfg.compare {
        info!("compare is true!")
    }

    let node_size = mem::size_of::<search::Node>();
    let board_size = mem::size_of::<Board>();
    let board_arr_size = mem::size_of::<[u8; MAX_BOARD_SIZE]>();

    info!("node size {node_size} bytes");
    info!("board size {board_size} bytes");
    info!("board_arrs size {board_arr_size} bytes");
}

#[bench]
fn playout_bench(b: &mut Bencher) {
    let mut cfg = release_config();

    log_test_init();

    cfg.max_boards = 1000;

    let res = env::var("COMPARE").unwrap_or("0".to_string());
    cfg.compare = str::parse::<u32>(&res).unwrap() == 1;

    print_info(&cfg);

    info!("running benchmark...");

    let game = test_game();
    let start_board = Board::from_str(BOARD, &game).unwrap();

    let mut ctx = ThreadContext::<FastRand>::new();
    let mut total_turns = 0;

    let start_time = Instant::now();

    b.iter(|| {
        ctx.board = start_board.clone();
        let (_, turns) = search::playout_game(&cfg, &mut ctx, &game);
        total_turns += turns;
    });

    let duration_ns = (Instant::now() - start_time).as_nanos();
    let ns_turn = duration_ns as f64 / total_turns as f64;

    info!("avg ns per turn {ns_turn:.3}")
}

#[bench]
fn search_bench(b: &mut Bencher) {
    let mut cfg = release_config();

    log_test_init();

    cfg.max_boards = 20000;
    cfg.num_threads = 8;
    cfg.latency = 0;
    cfg.fixed_iter = 4000;

    let res = env::var("COMPARE").unwrap_or("0".to_string());
    cfg.compare = str::parse::<u32>(&res).unwrap() == 1;

    print_info(&cfg);

    info!("allocating...");

    let ctx = Arc::new(SearchContext::<FastRand>::new(&cfg));
    let pool = Arc::new(ThreadPool::new(ctx.config.num_threads));

    let game = Arc::new(test_game());
    let board = Arc::new(Board::from_str(BOARD, &game).unwrap());

    info!("running benchmark...");

    b.iter(|| search::mcts(ctx.clone(), &pool, &board, &game, Instant::now()).unwrap());
}
