use conesnake::board::Board;
use conesnake::game::Map;
use conesnake::log::log_init;
use conesnake::pool::ThreadPool;
use conesnake::search;
use conesnake::tests::common::{get_context, wrapped_game};

use log::info;

use std::{sync::Arc, time::Instant};

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

fn main() {
    log_init();

    let ctx = Arc::new(get_context());
    let pool = ThreadPool::new(ctx.config.num_threads);

    let mut game = wrapped_game();
    let board = Board::from_str(ARCADE_MAZE_PROFILE_BOARD, &game).unwrap();
    game.api.map = Map::ArcadeMaze;

    for _ in 0..10 {
        let search_result = search::search_moves(ctx.clone(), &pool, &board, &game, Instant::now());
        let best_move = search::best_move(&search_result.scores);

        info!("snake eliminated if move left, move is {:?}", best_move);
    }
    drop(ctx)
}
