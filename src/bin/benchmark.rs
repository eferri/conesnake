use conesnake::board::Board;
use conesnake::config::Config;
use conesnake::log::log_init;
use conesnake::rand::FastRand;
use conesnake::search::{self, ThreadContext};
use conesnake::tests::common::test_game;

use std::process;
use std::time::Instant;

use clap::Parser;
use log::info;

const BOARD: &str = "
    turn: 1 health: 100 health: 100 health: 100 health: 100
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - 0 - - - - - 1 - -
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - 2 - - - - - 3 - -
    - - - - - - - - - - -
    - - - - - - - - - - -";

fn main() {
    let mut cfg = Config::parse();

    log_init();

    cfg.max_boards = 1000;
    cfg.max_snakes = 4;
    cfg.max_width = 10;
    cfg.max_height = 10;

    ctrlc::set_handler(move || {
        println!("Ctrl-C caught, exiting...");
        process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    info!("allocating...");

    #[cfg(feature = "simd")]
    info!("using simd");

    if cfg.compare {
        info!("compare is true!")
    }

    let game = test_game();
    let start_board = Board::from_str(BOARD, &game).unwrap();

    let num_rep = 30;
    let num_games = 7000;

    let mut run_times = Vec::new();
    let mut turns = Vec::new();

    for i in 0..num_rep {
        let mut num_total_turns = 0;

        info!("Starting rep {}", i);

        let start_time = Instant::now();
        let mut ctx = ThreadContext::<FastRand>::new(&cfg);

        for _ in 0..num_games {
            ctx.board = start_board.clone();
            let (_, num_turns) = search::playout_game(&cfg, &mut ctx, &game);
            num_total_turns += num_turns;
        }

        let duration = (Instant::now() - start_time).as_nanos();

        run_times.push(duration);
        turns.push(num_total_turns);
    }

    run_times.sort();

    if !turns.windows(2).all(|x| x[0] == x[1]) {
        info!("Benchmark was non-deterministic");
    }

    let min_duration = run_times[0] as f64 / 1.0e9;
    let max_duration = run_times[num_rep - 1] as f64 / 1.0e9;

    let med_duration = run_times[num_rep.div_ceil(2) - 1] as f64 / 1.0e9;

    let min_games_hz = num_games as f64 / max_duration;
    let max_games_hz = num_games as f64 / min_duration;
    let med_games_hz = num_games as f64 / med_duration;

    let min_turns_hz = turns[0] as f64 / max_duration;
    let max_turns_hz = turns[0] as f64 / min_duration;
    let med_turns_hz = turns[0] as f64 / med_duration;

    info!("");
    info!("Num games: {}", num_games);
    info!("Num turns: {}", turns[0]);
    info!("Avg Turns/Game: {:.2}", turns[0] as f64 / num_games as f64);
    info!("");
    info!("Min duration: {:.6}s", min_duration);
    info!("Max duration: {:.6}s", max_duration);
    info!("Med duration: {:.6}s", med_duration);
    info!("");
    info!("Min games per second: {:.4}", min_games_hz);
    info!("Max games per second: {:.4}", max_games_hz);
    info!("");
    info!("Min turns per second: {:.4}", min_turns_hz);
    info!("Max turns per second: {:.4}", max_turns_hz);
    info!("");
    info!("Med games per second: {:.4}", med_games_hz);
    info!("Med turns per second: {:.4}", med_turns_hz);
}
