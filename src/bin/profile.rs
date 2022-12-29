use conesnake::board::{Board, BoardSquare};
use conesnake::game::Map;
use conesnake::log::log_init;
use conesnake::pool::ThreadPool;
use conesnake::search;
use conesnake::tests::common::{get_context, wrapped_game};

use clap::Parser;
use log::info;

use std::{sync::Arc, time::Instant};

const HAZARD_ISLAND_PROFILE_BOARD: &str = "
    turn: 22 health: 80 health: 84 health: 89 health: 100
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
    * * - 1 * * * - - * *
";

const ARCADE_MAZE_PROFILE_BOARD: &str = "
    turn: 312 health: 45 health: 32 health: 89 health: 51
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
    * - * * * * * * * * * * * * * * * - *
";

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about = "profile", long_about = None)]
struct Args {
    #[clap(long)]
    pub arcade: bool,
}

fn main() {
    log_init();
    let ctx = Arc::new(get_context());

    let square_size = std::mem::size_of::<BoardSquare>();

    info!("size of BoardSquare : {} B", square_size);
    info!(
        "size of Board : {} B",
        square_size as i32 * ctx.config.max_width * ctx.config.max_height
    );

    let args = Args::parse();
    let pool = ThreadPool::new(ctx.config.num_worker_threads);

    let num_search = 20;

    let mut game = wrapped_game();

    let board = if args.arcade {
        game.api.map = Map::ArcadeMaze;
        Board::from_str(ARCADE_MAZE_PROFILE_BOARD, &game).unwrap()
    } else {
        game.api.map = Map::HzIslandsBridges;
        Board::from_str(HAZARD_ISLAND_PROFILE_BOARD, &game).unwrap()
    };

    let mut total_node_res = Vec::new();
    let mut total_node_sum = 0;

    for _ in 0..num_search {
        let search_result = search::search_moves(ctx.clone(), &pool, &board, &game, Instant::now()).unwrap();
        let best_move = search::best_move(&search_result.scores);

        total_node_res.push(search_result.total_nodes);
        total_node_sum += search_result.total_nodes;

        if args.arcade {
            info!("snake eliminated if move left, move is {}", best_move);
        }
    }

    total_node_res.sort();

    info!("********** RESULTS **********");
    info!("Mean total nodes: {}", total_node_res[num_search / 2]);
    info!("Average total nodes: {}", total_node_sum as f64 / num_search as f64);

    drop(ctx)
}
