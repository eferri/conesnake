use crate::search;

use crate::board::Board;
use crate::game::Map;
use crate::log::log_test_init;
use crate::pool::ThreadPool;

use crate::tests::common::{get_context, solo_game, test_game, wrapped_game};
use crate::util::Move;

use approx::assert_relative_eq;
use log::info;

use std::sync::{atomic::Ordering, Arc};
use std::time::Instant;

#[test]
fn expand_node_test() {
    let ctx = get_context();
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
                    ^ 1 < -
                    ^ - ^ d
                    ^ l ^ <",
                    vec![Move::Right, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 0
                    > v - -
                    ^ 0 - -
                    ^ - - -
                    ^ l - -",
                    vec![Move::Down, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 92
                    > > 0 -
                    ^ - > 1
                    ^ - ^ d
                    ^ l ^ <",
                    vec![Move::Right, Move::Right],
                ),
                (
                    "turn: 3 health: 44 health: 92
                    > v - -
                    ^ 0 > 1
                    ^ - ^ d
                    ^ l ^ <",
                    vec![Move::Down, Move::Right],
                ),
                (
                    "turn: 3 health: 44 health: 0
                    > > 0 -
                    ^ - - -
                    ^ - - -
                    ^ l - -",
                    vec![Move::Right, Move::Up],
                ),
                (
                    "turn: 3 health: 44 health: 92
                    > v 1 -
                    ^ 0 ^ -
                    ^ - ^ d
                    ^ l ^ <",
                    vec![Move::Down, Move::Up],
                ),
            ],
        ),
        (
            "turn: 2 health: 45 health: 93
            > v - -
            ^ v 1 d
            ^ 0 ^ v
            ^ L ^ <",
            vec![
                (
                    "turn: 3 health: 0 health: 92
                    - - - -
                    - - > 1
                    - - ^ d
                    - - ^ <",
                    vec![Move::Left, Move::Right],
                ),
                (
                    "turn: 3 health: 0 health: 92
                    - - 1 -
                    - - ^ -
                    - - ^ d
                    - - ^ <",
                    vec![Move::Left, Move::Up],
                ),
            ],
        ),
        (
            "turn: 2 health: 45  health: 0 health: 100
            > 0 - -
            ^ - - -
            ^ - 2 D
            ^ l ^ <",
            vec![
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > > 0 -
                    ^ - - -
                    ^ 2 < d
                    u - ^ <
                    ",
                    vec![Move::Right, Move::Left, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > v - -
                    ^ 0 - -
                    ^ 2 < d
                    u - ^ <
                    ",
                    vec![Move::Down, Move::Left, Move::Left],
                ),
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > > 0 -
                    ^ - 2 -
                    ^ - ^ d
                    u - ^ <
                    ",
                    vec![Move::Right, Move::Left, Move::Up],
                ),
                (
                    "turn: 3 health: 44 health: 0 health: 99
                    > v - -
                    ^ 0 2 -
                    ^ - ^ d
                    u - ^ <
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
            - - - - - v - u
            - - - - - > > 2",
            vec![
                (
                    "turn: 4 health: 96 health: 96 health: 96
                    > > > > 1 - - -
                    ^ - - - - - - -
                    ^ - - d d v < <
                    ^ < < < v v - ^
                    - - - - 0 v - u
                    - - - - - v - 2
                    - - - - - > > ^",
                    vec![Move::Down, Move::Right, Move::Up],
                ),
                (
                    "turn: 4 health: 96 health: 96 health: 96
                    > > > v - - - -
                    ^ - - 1 - - - -
                    ^ - - d d v < <
                    ^ < < < v v - ^
                    - - - - 0 v - u
                    - - - - - v - 2
                    - - - - - > > ^",
                    vec![Move::Down, Move::Down, Move::Up],
                ),
            ],
        ),
    ];

    let scratch_guard = ctx.thread_scratch.read().unwrap();
    let mut state = scratch_guard[0].write().unwrap();
    let space_guard = ctx.node_space.read().unwrap();
    let mut root_state_guard = space_guard[0].state.write().unwrap();

    for (start_board, expected_results) in &test_cases {
        let start_board = Board::from_str(start_board, &game).unwrap();

        ctx.reset();
        root_state_guard.reset();
        root_state_guard.board = start_board;
        ctx.total_nodes.fetch_add(1, Ordering::AcqRel);

        search::expand_node(&ctx, &mut state, &mut root_state_guard, 0, &test_game());

        assert_eq!(root_state_guard.num_children, expected_results.len());

        for (idx, (board, moves)) in expected_results.iter().enumerate() {
            // Ignore moves of snakes that were dead before expanding
            for (snake_idx, snake) in root_state_guard.board.snakes.iter().enumerate() {
                if snake.alive {
                    assert_eq!(
                        moves[snake_idx],
                        root_state_guard.child_moves(idx as usize)[snake_idx],
                        "Expected moves: {:?}, actual moves: {:?}",
                        moves[snake_idx],
                        root_state_guard.child_moves(idx as usize)
                    );
                }
            }

            assert_eq!(root_state_guard.children[idx].index, idx + 1);

            assert_eq!(
                space_guard[idx + 1].state.read().unwrap().board,
                Board::from_str(board, &game).unwrap()
            );
        }
    }
}

// Should loose in 3 moves
const PLAYOUT_TRAPPED: &str = "
    turn: 2 health: 45 health: 93
    > 0 v <
    ^ - v U
    ^ - > 1
    ^ < < L
";

const PLAYOUT_WIN: &str = "
    turn: 2 health: 45 health: 0
    > 0 - -
    ^ - - -
    ^ - - -
    ^ < < L
";

fn check_scores(exp_score: &[f64], act_score: &[f64]) {
    for (i, score) in exp_score.iter().enumerate() {
        assert_relative_eq!(score, &act_score[i], epsilon = f64::EPSILON);
    }
}

#[test]
fn playout_test() {
    let ctx = get_context();
    let game = test_game();

    let scratch_space_guard = ctx.thread_scratch.write().unwrap();
    let mut scratch_guard = scratch_space_guard[0].write().unwrap();

    let start_board = Board::from_str(PLAYOUT_TRAPPED, &game).unwrap();

    scratch_guard.play_scores.clear();
    scratch_guard.board = start_board;

    search::playout_game(&ctx, &mut scratch_guard, &game, 0);

    assert!(scratch_guard.play_scores.len() == 2);
    check_scores(&scratch_guard.play_scores, &[1.0, 0.0]);

    let start_board = Board::from_str(PLAYOUT_WIN, &game).unwrap();
    scratch_guard.play_scores.clear();
    scratch_guard.board = start_board;

    search::playout_game(&ctx, &mut scratch_guard, &game, 0);

    assert!(scratch_guard.play_scores.len() == 2);
    check_scores(&scratch_guard.play_scores, &[1.0, 0.0]);
}

const SEARCH_SMALL: &str = "
    turn: 2 health: 45
    > 0 +
    ^ v <
    ^ < u
";

#[test]
fn small_search_test() {
    log_test_init();
    let ctx = get_context();
    let pool = ThreadPool::new(ctx.config.num_threads);

    let mut game = solo_game();

    game.add_board(Board::from_str(SEARCH_SMALL, &game).unwrap());

    {
        let mut guard = ctx.game.write().unwrap();
        *guard = Some(game);
    }

    let search_result = search::best_move(Arc::new(ctx), &pool, Instant::now(), 0);

    assert_eq!(search_result.best_move, Move::Right);
    assert_eq!(search_result.max_depth, 2);
    assert_eq!(search_result.total_nodes, 3);
}

const SEARCH_HEAD_ON: &str = "
    turn: 2 health: 45 health: 34
    v l - - - - - - - -
    v - - - - - - - - -
    0 - 1 - - - - - - -
    - - ^ - - - - - - -
    - - ^ < < l - - - -
    - - - - - - - - - -
    - - - - - - - - - -
    - - - - - - - - - -
    - - - - - - - - - -
";

#[test]
fn head_on_search_test() {
    log_test_init();
    let ctx = get_context();
    let pool = ThreadPool::new(ctx.config.num_threads);
    let mut game = test_game();

    game.add_board(Board::from_str(SEARCH_HEAD_ON, &game).unwrap());
    {
        let mut guard = ctx.game.write().unwrap();
        *guard = Some(game);
    }

    let search_result = search::best_move(Arc::new(ctx), &pool, Instant::now(), 0);

    assert_ne!(search_result.best_move, Move::Right);
}

const ARCADE_MAZE_BOARD: &str = "
    turn: 3 health: 97 health: 97 health: 97
    * - * * * * * * * * * * * * * * * v *
    * - - - - - - - - * - - - - - - - 1 *
    * - * * - * * * - * - * * * - * * - *
    * 0 < l - - - - - - - - - - - - - - *
    * - * * - * - * * * * * - * - * * - *
    * - - - - * - - - * - - - * - - - - *
    * - - * - * * * - * - * * * - * - - *
    * - - * - * - - - - - - - * - * - - *
    * * * * - * - * - * - * - * - * * * *
    - - - - v l - * - - - * - - - - - - -
    * * * * 2 * - * - * - * - * - * * * *
    * - - * - * - - - - - - - * - * - - *
    * - - * - * - * * * * * - * - * - - *
    * - - - - - - - - * - - - - - - - - *
    * - * * - * * * - * - * * * - * * - *
    * - - * - - - - - - - - - - - * - - *
    * * - * - * - * * * * * - * - * - * *
    * - - - - * - - - * - - - * - - - - *
    * - * * * * * * - * - * * * * * * - *
    * - - - - - - - - - - - - - - - - - *
    * - * * * * * * * * * * * * * * * d *
";

#[test]
fn arcade_maze_search_test() {
    log_test_init();
    let ctx = Arc::new(get_context());
    let pool = ThreadPool::new(ctx.config.num_threads);

    for _ in 0..4 {
        let mut game = wrapped_game();
        game.api.map = Map::ArcadeMaze;
        game.add_board(Board::from_str(ARCADE_MAZE_BOARD, &game).unwrap());
        {
            let mut guard = ctx.game.write().unwrap();
            *guard = Some(game);
        }

        let search_result = search::best_move(ctx.clone(), &pool, Instant::now(), 0);

        assert!(search_result.best_move == Move::Down || search_result.best_move == Move::Up);
    }
}

const ARCADE_MAZE_PROFILE_BOARD: &str = "
    turn: 312 health: 45 health: 32 health: 89 health: 51
    * - * * * * * * * * * * * * * * * - *
    * - - - - - - - - * - - - - v < < l *
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
    * * v * u * - * * * * * ^ * - * - * *
    * - > > 1 * r > v * > > ^ * - - - - *
    * - * * * * * * v * ^ * * * * * * - *
    * + - - - - - - > > ^ - - - - - - + *
    * - * * * * * * * * * * * * * * * - *
";

#[test]
fn arcade_maze_profile_test() {
    log_test_init();

    let ctx = Arc::new(get_context());
    let pool = ThreadPool::new(ctx.config.num_threads);

    let mut game = wrapped_game();
    game.api.map = Map::ArcadeMaze;
    game.add_board(Board::from_str(ARCADE_MAZE_PROFILE_BOARD, &game).unwrap());
    {
        let mut guard = ctx.game.write().unwrap();
        *guard = Some(game);
    }

    #[cfg(feature = "profile")]
    let _prof = -1;

    let search_result = search::best_move(ctx.clone(), &pool, Instant::now(), 15);

    info!("snake eliminated if move left, move is {:?}", search_result.best_move);

    drop(ctx)
}
