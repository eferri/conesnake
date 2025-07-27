use crate::search;

use crate::board::{Board, Snake};
use crate::log::log_test_init;
use crate::pool::ThreadPool;
use crate::tests::common::{get_context, get_deterministic_context, solo_game, test_game, wrapped_game};
use crate::util::Move;

use approx::assert_relative_eq;
use pretty_assertions::assert_eq;

#[cfg(feature = "simd")]
use std::simd::num::SimdFloat;

use std::sync::{atomic::Ordering, Arc};
use std::time::Instant;

#[test]
fn expand_node_test() {
    let ctx = get_deterministic_context();
    let game = test_game();

    let test_cases = [
        (
            "turn: 2 health: 45 health: 93
            > 0 - -
            ^ - 1 d
            ^ d ^ v
            ^ < ^ <",
            vec![
                (
                    "turn: 3 health: 44 health: 92
                    > > 0 -
                    ^ 1 < +
                    ^ - ^ d
                    ^ a ^ <",
                    vec![Move::Right, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 0
                    > v - -
                    ^ 0 - -
                    ^ - + -
                    ^ a - -",
                    vec![Move::Down, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 92
                    > > 0 -
                    ^ + > 1
                    ^ - ^ d
                    ^ a ^ <",
                    vec![Move::Right, Move::Right],
                ),
                (
                    "turn: 3 health: 44 health: 92
                    > v + -
                    ^ 0 > 1
                    ^ - ^ d
                    ^ a ^ <",
                    vec![Move::Down, Move::Right],
                ),
                (
                    "turn: 3 health: 44 health: 0
                    > > 0 -
                    ^ + - -
                    ^ - - -
                    ^ a - -",
                    vec![Move::Right, Move::Up],
                ),
                (
                    "turn: 3 health: 44 health: 92
                    > v 1 -
                    ^ 0 ^ +
                    ^ - ^ d
                    ^ a ^ <",
                    vec![Move::Down, Move::Up],
                ),
            ],
        ),
        (
            "turn: 2 health: 45 health: 93
            > v - -
            ^ v 1 d
            ^ 0 ^ v
            ^ e ^ <",
            vec![
                (
                    "turn: 3 health: 0 health: 92
                    - - - -
                    - - > 1
                    + - ^ d
                    - - ^ <",
                    vec![Move::Left, Move::Right],
                ),
                (
                    "turn: 3 health: 0 health: 92
                    - - 1 -
                    - - ^ -
                    + - ^ d
                    - - ^ <",
                    vec![Move::Left, Move::Up],
                ),
            ],
        ),
        (
            "turn: 2 health: 45  health: 0 health: 100
            > 0 - -
            ^ - - -
            ^ - 2 h
            ^ a ^ <",
            vec![
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > > 0 -
                    ^ - - +
                    ^ 2 < d
                    c  - ^ <
                    ",
                    vec![Move::Right, Move::Left, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > v - -
                    ^ 0 - +
                    ^ 2 < d
                    c  - ^ <
                    ",
                    vec![Move::Down, Move::Left, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > > 0 -
                    ^ - 2 -
                    ^ + ^ d
                    c  - ^ <
                    ",
                    vec![Move::Right, Move::Left, Move::Up],
                ),
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > v - +
                    ^ 0 2 -
                    ^ - ^ d
                    c  - ^ <
                    ",
                    vec![Move::Down, Move::Left, Move::Up],
                ),
            ],
        ),
        (
            "turn: 3 health: 97 health: 97 health: 97
            > > > 1 - - - -
            ^ - - d d - - -
            ^ - - v v v < <
            ^ < < < 0 v - ^
            - - - - - v - ^
            - - - - - v - c
            - - - - - > > 2",
            vec![
                (
                    "turn: 4 health: 96 health: 96 health: 96
                    > > > > 1 - - -
                    ^ - - - - - - -
                    ^ - - d d v < <
                    ^ < < < v v - ^
                    - - - - 0 v - c
                    + - - - - v - 2
                    - - - - - > > ^",
                    vec![Move::Down, Move::Right, Move::Up],
                ),
                (
                    "turn: 4 health: 96 health: 96 health: 96
                    > > > v - - - -
                    ^ - - 1 - - - -
                    ^ - - d d v < <
                    ^ < < < v v - ^
                    - - - - 0 v - c
                    + - - - - v - 2
                    - - - - - > > ^",
                    vec![Move::Down, Move::Down, Move::Up],
                ),
            ],
        ),
    ];

    let mut thread_state_guard = ctx.thread_state[0].lock().unwrap();
    let mut root_node_guard = ctx.node_space[0].write().unwrap();

    for (start_board, expected_results) in &test_cases {
        let start_board = Board::from_str(start_board, &game).unwrap();

        ctx.reset();
        root_node_guard.reset();
        root_node_guard.board = start_board;

        ctx.total_nodes.fetch_add(1, Ordering::AcqRel);

        while !root_node_guard.is_fully_expanded() {
            search::expand_node(&ctx, &test_game(), &mut thread_state_guard, &mut root_node_guard, 0).unwrap();
        }

        assert_eq!(root_node_guard.num_children as usize, expected_results.len());

        for (idx, (board, exp_moves)) in expected_results.iter().enumerate() {
            // Ignore moves of snakes that were dead before expanding
            let mut act_moves = Move::decode(root_node_guard.child_moves[idx], root_node_guard.board.num_snakes());

            #[allow(clippy::needless_range_loop)]
            for snake_idx in 0..root_node_guard.board.num_snakes() as usize {
                if !root_node_guard.board.snakes[snake_idx].alive() {
                    act_moves[snake_idx] = Move::Left;
                }
            }

            assert_eq!(
                *exp_moves, act_moves,
                "Expected moves: {:?}, actual moves: {:?}",
                *exp_moves, act_moves
            );

            // Ignore status of snakes that are dead, not encoded in string
            {
                let mut state_guard = ctx.node_space[idx + 1].write().unwrap();
                for snake_idx in 0..state_guard.board.num_snakes() as usize {
                    let snake = &mut state_guard.board.snakes[snake_idx];
                    if !snake.alive() {
                        *snake = Snake::new();
                    }
                }
            }

            let mut compare_board = Board::new(0, 0);
            compare_board.set_from(&ctx.node_space[idx + 1].read().unwrap().board);

            assert_eq!(compare_board, Board::from_str(board, &game).unwrap());
        }
    }
}

// Should loose in 3 moves
const PLAYOUT_TRAPPED: &str = "
    turn: 2 health: 45 health: 93
    > 0 v <
    ^ - v g
    ^ - > 1
    ^ < < e
";

const PLAYOUT_WIN: &str = "
    turn: 2 health: 45 health: 0
    > 0 - -
    ^ - - -
    ^ - - -
    ^ < < e
";

fn check_scores(exp_score: &[f64], act_score: &[f64]) {
    for (i, score) in exp_score.iter().enumerate() {
        assert_relative_eq!(score, &act_score[i], epsilon = f64::EPSILON);
    }
}

#[test]
fn playout_test() {
    let ctx = get_deterministic_context();
    let game = test_game();

    let mut thread_state_guard = ctx.thread_state[0].lock().unwrap();
    let start_board = Board::from_str(PLAYOUT_TRAPPED, &game).unwrap();

    thread_state_guard.board = start_board;

    search::playout_game(&ctx.config, &mut thread_state_guard, &game);

    check_scores(
        &thread_state_guard.play_scores[0..2],
        &[ctx.config.win_val, ctx.config.loss_val],
    );

    let start_board = Board::from_str(PLAYOUT_WIN, &game).unwrap();
    thread_state_guard.board = start_board;

    search::playout_game(&ctx.config, &mut thread_state_guard, &game);

    check_scores(
        &thread_state_guard.play_scores[0..2],
        &[ctx.config.win_val, ctx.config.loss_val],
    );
}

const SEARCH_SMALL: &str = "
    turn: 2 health: 45
    > 0 +
    ^ v <
    ^ < c
";

#[test]
fn small_search_test() {
    log_test_init();
    let ctx = Arc::new(get_deterministic_context());
    let pool = ThreadPool::new(ctx.config.num_threads);

    let game = solo_game();
    let board = Board::from_str(SEARCH_SMALL, &game).unwrap();

    let search_result = search::mcts(ctx.clone(), &pool, &board, &game, Instant::now()).unwrap();
    let best_move = search::best_move(&ctx.config, 0, &search_result.scores, true);

    assert_eq!(best_move, Move::Right);
    assert_eq!(search_result.max_depth, 2);
    assert_eq!(search_result.total_nodes, 3);
}

const MAZE_BOARD: &str = "
    turn: 3 health: 97 health: 97 health: 0
    * - * * - * * * * * *
    * - - - - - - - - * *
    * - * * - * * * - * *
    * 0 < a - - - - - - -
    * - * * - * - * * * *
    * - - - - * - - - * *
    * - - * - * * * - * *
    * - - * - * - - - - *
    * * * * - * - * - * *
    - - - - v a - * - - *
    * * * * 1 * - * - * *
";

#[test]
fn arcade_maze_search_test() {
    log_test_init();
    let ctx = Arc::new(get_context());
    let pool = ThreadPool::new(ctx.config.num_threads);

    for _ in 0..4 {
        let game = wrapped_game();
        let board = Board::from_str(MAZE_BOARD, &game).unwrap();

        let search_result = search::mcts(ctx.clone(), &pool, &board, &game, Instant::now()).unwrap();
        let best_move = search::best_move(&ctx.config, 0, &search_result.scores, true);

        assert!(best_move == Move::Down || best_move == Move::Up);

        // Ensure simd duct score produces same result as non-simd version
        #[cfg(feature = "simd")]
        {
            let root_guard = ctx.node_space[0].read().unwrap();

            for child_moves in root_guard.child_moves[0..root_guard.num_children as usize].iter() {
                let mut duct_sum = 0.0;

                for snake_idx in 0..root_guard.board.num_snakes() as usize {
                    if !root_guard.board.snakes[snake_idx].alive() {
                        continue;
                    }
                    let mv = Move::extract(*child_moves, snake_idx as u32);
                    duct_sum += root_guard.duct_score(&ctx.config, &game, snake_idx, mv)
                }

                let duct_sum_simd = root_guard
                    .duct_scores_simd(&ctx.config, &game, *child_moves)
                    .reduce_sum();

                assert_relative_eq!(duct_sum, duct_sum_simd as f64, epsilon = 1e-5);
            }
        }
    }
}
