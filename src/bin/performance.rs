use conesnake::board::Board;
use conesnake::config::Config;
use conesnake::game::{Map, Rules};
use conesnake::pool::ThreadPool;
use conesnake::rand::FastRand;
use conesnake::search;
use conesnake::search::SearchContext;
use conesnake::tests::common::test_game;
use conesnake::util::{Error, Move};

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Results {
    failures: i64,
    loss: f64,
    games: i64,
    cases: i64,
    median_nodes: i64,
    mean_nodes: f64,
    max_nodes: i64,
}

type TestCase<'a> = &'a [(usize, bool, Move)];

const TESTS: &[(&str, TestCase, Rules, Map)] = &[
    (
        "turn: 2 health: 45 health: 34
         v a - - - - - - - -
         v - - - - - - - - -
         0 - 1 - - - - - - -
         - - ^ - - - - - - -
         - - ^ < < a - - - -
         - - - - - - - - - -
         - - - - - - - - - -
         - - - - - - - - - -
         - - - - - - - - - -",
        &[(0, true, Move::Down), (1, true, Move::Left)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 232 health: 81 health: 45 health: 58 health: 42
        - - + > > > > v - - -
        - - - ^ < < a 3 - b v
        - d - + - - - - - - v
        - v - - - - - 0 < < v
        - > v - - v < < < ^ <
        - - > v v < - b ^ - -
        - - 1 v v - 2 - - - -
        - - ^ v > > ^ - - - -
        - - ^ < - - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -",
        &[(0, true, Move::Left)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 312 health: 45 health: 32 health: 89 health: 51
        * - * * * * * * * * * * * * * * * - *
        * - - - - - - - - * - - - - v < < a *
        * - * * - * * * - * - * * * v * * - *
        * - - - > 2 - - - + - - - - v - - - *
        * - * * ^ * - * * * * * - * v * * - *
        * - - - ^ * - - d * - - - * v - - - *
        * - - * ^ * * * v * - * * * v * - - *
        * - - * ^ * v < < - - - - * v * - - *
        * * * * ^ * v * - * - * - * v * * * *
        - - - - ^ < < * - + - * - - v - - - -
        * * * * - * - * - * - * - * v * * * *
        * v < * - * - - - - - - - * v * - - *
        * v ^ * - * - * * * * * - * 3 * - - *
        * v ^ < < - - - - * - - - - - - - - *
        * v * * ^ * * * - * - * * * - * * - *
        * > v * ^ - 0 < < < < < < - - * - - *
        * * v * c * - * * * * * ^ * - * - * *
        * - > > 1 * b > v * > > ^ * - - - - *
        * - * * * * * * v * ^ * * * * * * - *
        * + - - - - - - > > ^ - - - - - - + *
        * - * * * * * * * * * * * * * * * - *",
        &[(0, true, Move::Left)],
        Rules::Wrapped,
        Map::ArcadeMaze,
    ),
    (
        "turn: 22 health: 80 health: 84 health: 89 health: 100
        * * - ^ * * * - - * *
        * - - ^ a * F > v - *
        - - - - - + - - 3 - -
        - - - - - * - - - - -
        * + 2 - - * - - - - *
        * * ^ * * * * * - * *
        * - ^ - - * - - - - *
        - - ^ - - * - - - v a
        - - c - - - - - 0 < -
        * - - - - * - - - - *
        * * - 1 * * * - - * *",
        &[],
        Rules::Wrapped,
        Map::HzIslandsBridges,
    ),
    (
        "turn: 55 health: 98 health: 47 health: 68 health: 90
        - - - > > > v - - - -
        - - - ^ < - 0 - - - -
        - - - - c - - 1 d - -
        - - - - - - - ^ < - -
        - - - - v < < - - - -
        - - - - 3 - ^ - - - -
        - - - - - - ^ - - - -
        - - - + d - c - - - -
        - - - - > v - - - - -
        - - - - 2 < - - - - -
        - - - - - - - - - - -",
        &[(0, false, Move::Left)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 158 health: 97 health: 54 health: 36
        - - - - - - - - - - -
        - > > 0 - - > 2 - - -
        - ^ v a - - ^ - - - -
        + ^ v 1 < < ^ - - - -
        > ^ v - > ^ c - - - -
        ^ < v - ^ - - - - - -
        - c > > ^ - + - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -",
        &[(0, false, Move::Down), (1, true, Move::Up)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 33 health: 75 health: 84 health: 96 health: 99
        - - - - - - - - - - -
        - - - - - - - - - - -
        - - - - v < < a - - -
        - - - - > v - > 1 - -
        - - - - - 2 - ^ < - -
        - - v < < a - - c - -
        - - v - - - - - - - -
        - - 3 - - - - - - - -
        - 0 - - - - - - - - -
        - ^ < - - - - - - - -
        - b ^ - - + - - - - -",
        &[(0, true, Move::Left), (3, false, Move::Right)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 30 health: 85 health: 96 health: 80 health: 98
        + - 2 < - - - - - - -
        - - - ^ < a - - - - -
        - - - - - - - - - - -
        - - - 1 - - - d - - -
        - - - ^ d - - v - - -
        - - - ^ v 0 < < - - -
        - - - ^ < - - - 3 - -
        - - - - - b > > ^ - -
        - - - - - - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -",
        &[(0, true, Move::Down)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 113 health: 26 health: 95 health: 100 health: 83
        - - + 0 < < < a - - -
        - - - - - - - - - - -
        - - - 3 < < - - - - -
        - - - b v ^ - - - - -
        - - - - v ^ - b > v -
        - - - - > ^ - - - > v
        - - - - - 1 < < < < <
        - - f > > v - - - - -
        - - - - - > > > v - -
        - - - - - - - - 2 - -
        - - - - - - - - - - -
        ",
        &[(0, true, Move::Left), (3, true, Move::Up)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 147 health: 54 health: 4 health: 92 health: 87
        - - - - > v - - - - -
        - - - - c v - - - - -
        d - - - - 0 v < - - -
        v - 1 - - - 2 ^ - - -
        > v ^ - - - - ^ < < <
        - > ^ - - b > > > > ^
        - + v < - - - - - - -
        - - 3 ^ < < < < < < -
        - - - - - - - - > ^ -
        - - - - - - - - c - -
        - - - - - - - - - - -",
        &[(0, true, Move::Left), (2, true, Move::Left)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 242 health: 10 health: 78 health: 84
        - - 1 < - - + - - v <
        - + - ^ - + - - - v ^
        - - - ^ - - - - - v ^
        - - - ^ < < < < - 0 ^
        - - - - - b > ^ - - ^
        - - - - - v < - - - ^
        - - - - 2 < ^ < < - c
        - - - b v > > > ^ - -
        - - - - > ^ - - - - -
        - - - - - - - - - - -
        - - + - - - - - - - -",
        &[(0, true, Move::Left)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 10 health: 92 health: 98 health: 92 health: 92
        - - - - - - - - - - -
        - - - - - - - - - - -
        - - - - - d - - - - -
        - - - - - > v - - - -
        - - + - - - 2 - - - -
        v < - - - > > 1 - - -
        3 c - - - ^ a - - - +
        - 0 - - - - - - - - -
        > ^ - - - - - - - - -
        c - - + - - - - - - -
        - - - - - - - - - - -
        ",
        &[(0, true, Move::Right)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 129 health: 95 health: 93 health: 72 health: 83
        - - - b > > > > > > v
        - - - - - - - - - - 0
        - - - - - - b > > v -
        - - - - 2 < v a - v -
        - - - > > ^ v - - 3 -
        - - - ^ < - > v - - -
        - - - > ^ v < v - - +
        - - - c - v ^ v - - -
        - - - 1 v < ^ < - - -
        - - - ^ < - - - - - -
        - - - - - - - - - + -
        ",
        &[(0, true, Move::Left), (3, true, Move::Right)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 74 health: 53 health: 97 health: 56 health: 100
        - > > > 1 - - - - - -
        - ^ < < a - - - - - -
        - - - - - - - > 3 - -
        - - - - - - - ^ - - -
        - - d - - f > ^ - - -
        - v < - - - - - - - -
        - v + d > > 2 - - - -
        - 0 - > ^ - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -",
        &[(0, true, Move::Right)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        // Maybe unstable
        "turn: 204 health: 46 health: 84 health: 51 health: 80
        - - - - 2 < < - - - -
        - - - - > v ^ - d > v
        - - + > ^ > ^ - v ^ v
        - - - ^ - - - 0 v ^ v
        - - - ^ - - - ^ v ^ v
        - - > ^ - - - ^ < ^ v
        - - c - 1 < < - - c 3
        - - > > v > ^ - - - -
        - - ^ < v ^ - - - - -
        + - - c > ^ - - - - -
        - - - - - - - - - - -",
        &[(0, true, Move::Left), (3, true, Move::Left)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 163 health: 13 health: 75 health: 17 health: 25
        + - - - - 3 < < < < <
        - - - - - - - - - - ^
        - - - - - - - - - - ^
        - - - v < a - - - - c
        - 2 < < - - - - - - -
        0 - - - - - 1 - - - -
        ^ - - d - > ^ - - - -
        ^ - - > v ^ - - - - -
        ^ < - - > ^ - - - - -
        > ^ - - - - - - - - -
        ^ < a - - - - - - - -",
        &[(0, true, Move::Right), (2, true, Move::Up)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 173 health: 82 health: 94 health: 80
        > > > 0 + - - - - - -
        ^ < - - - - - b v - -
        - c - - - - - - v - +
        - - v < v < < v < - -
        - - v ^ v - ^ < - - -
        - - v c 1 - - - - - -
        - + v - - - - - - - -
        - - > > 2 - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -
        - - - - - - - - - - -
        ",
        &[(0, true, Move::Right)],
        Rules::Standard,
        Map::Standard,
    ),
    (
        "turn: 280 health: 4 health: 76 health: 71
        - + - - - - - - - - -
        + - - - - - - - - - -
        - - - - v < < < - - -
        - - - v < - - ^ > > v
        - - - > 1 > > ^ ^ < v
        - - - - - ^ - - - c v
        - - - - - c - - - - v
        - - - > > > > 0 + - v
        - b > ^ - - - - 2 < <
        - - - - - - - - - - -
        - - - - - - - - - - -
        ",
        &[(0, true, Move::Up)],
        Rules::Standard,
        Map::Standard,
    ),
];

fn main() -> Result<(), Error> {
    let mut cfg = Config::parse();

    cfg.max_boards = 800000;
    cfg.max_snakes = 4;
    cfg.max_width = 19;
    cfg.max_height = 21;

    let done_flag = Arc::new(AtomicBool::new(false));
    let flag = done_flag.clone();

    ctrlc::set_handler(move || {
        println!("Ctrl-C caught, exiting...");
        flag.store(true, Ordering::Release);
    })
    .expect("Error setting Ctrl-C handler");

    if let Ok(t) = env::var("NUM_THREADS") {
        cfg.num_worker_threads = str::parse(&t).unwrap();
    }

    eprintln!("allocating...");

    let ctx = Arc::new(SearchContext::<FastRand>::new(&cfg));
    let pool = ThreadPool::new(ctx.config.num_worker_threads);

    let mut game = test_game();

    let mut total_node_res = Vec::new();
    let mut total_node_sum = 0;

    let mut results: Results = Default::default();

    let num_runs = 6;

    results.games = TESTS.len() as i64 * num_runs as i64;

    for run_idx in 0..num_runs {
        for (test_idx, (board_str, desired_moves, rules, map)) in TESTS.iter().enumerate() {
            if done_flag.load(Ordering::Acquire) {
                break;
            }

            eprintln!("run {} search {} / {}", run_idx, test_idx, TESTS.len());

            game.ruleset = *rules;
            game.api.map = *map;

            let board = Board::from_str(board_str, &game)?;

            let mut search_result = search::search_moves(ctx.clone(), &pool, &board, &game, Instant::now()).unwrap();

            if search_result.total_nodes > results.max_nodes {
                results.max_nodes = search_result.total_nodes;
            }

            total_node_res.push(search_result.total_nodes);
            total_node_sum += search_result.total_nodes;

            for (snake_idx, eq, mv) in *desired_moves {
                let actual_move = search::best_move(&search_result.scores[*snake_idx], true);
                let passed = *eq && actual_move == *mv || !*eq && actual_move != *mv;

                for score in &mut search_result.scores[*snake_idx] {
                    if score.games > 0 {
                        score.score /= score.games as f64;
                    }
                }

                if !passed {
                    let mut failure_str = format!(
                        "{}\nSnake {} Condition: mv{}{} Actual move: {}\nScores {:#?}\n",
                        board,
                        snake_idx,
                        if *eq { "==" } else { "!=" },
                        mv,
                        actual_move,
                        search_result.scores[*snake_idx]
                    );

                    if !board.valid_move(&game, *snake_idx, actual_move) {
                        failure_str.push_str("ERROR Actual move was invalid!")
                    }

                    eprintln!("{}", failure_str);
                    results.failures += 1;

                    results.loss += search_result.scores[*snake_idx][actual_move as usize].score
                        - search_result.scores[*snake_idx][*mv as usize].score;
                } else {
                    results.loss -= 0.05;
                }

                results.cases += 1;
            }
        }
    }

    total_node_res.sort();

    let total_tests = TESTS.len() * num_runs;

    results.median_nodes = total_node_res[total_tests / 2];
    results.mean_nodes = total_node_sum as f64 / total_tests as f64;

    println!("{}", serde_json::to_string(&results).unwrap());

    Ok(())
}
