use crate::api::{Scores, SearchStats};
use crate::board::Board;
use crate::config::Config;
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::rand::Rand;
use crate::util::{Coord, Error, Move};

use deepsize::DeepSizeOf;
use log::{error, info, warn};

use std::sync::{atomic::AtomicBool, atomic::AtomicI64, atomic::Ordering, Arc, Barrier};
use std::sync::{Mutex, RwLock};
use std::{thread::sleep, time::Duration, time::Instant};

// Global search resources shared among search threads
pub struct SearchContext<R: Rand> {
    pub config: Config,
    pub game: RwLock<Option<Game>>,

    search_lock: Mutex<()>,
    node_space: Vec<RwLock<Node>>,
    thread_state: Vec<Mutex<ThreadContext<R>>>,

    // Search state
    search_timeout: AtomicBool,
    done_barrier: Barrier,
    total_nodes: AtomicI64,
    num_games: AtomicI64,
    num_playouts: AtomicI64,
}

// Thread-local search resources, pre-allocated
pub struct ThreadContext<R: Rand> {
    rng: R,

    board: Board,
    food_buff: Vec<Coord>,

    play_scores: Vec<f64>,
}

// Node in search tree
#[derive(Clone, DeepSizeOf)]
pub struct Node {
    board: Board,

    parent_node_idx: u32,
    parent_move_idx: u32,

    depth: i32,
    max_depth: i32,

    virtual_loss: bool,

    games: i32,
    score: Vec<f64>,

    cache: Vec<[NodeScoreCache; 4]>,

    num_children: u32,
    num_move_perms: u32,
    // This uses a LOT of memory
    // each node has 4^max_snakes children
    children: Vec<NodePtr>,
}

#[derive(Default, Debug, Clone, DeepSizeOf)]
pub struct NodeScoreCache {
    score: f64,
    heuristic_score: f64,
    games: i64,
    pruned: bool,
}

// Pointer to child nodes, with corresponding moves
// 2-bit moves are encoded into "moves" u32 to save memory
// In Royale map, shrink direction is encoded in last 2 bits
#[derive(Clone, DeepSizeOf)]
pub struct NodePtr {
    moves: u32,
    index: u32,
}

impl<R: Rand> SearchContext<R> {
    pub fn new(config: &Config) -> Self {
        let mut node_space = Vec::with_capacity(config.max_boards);
        node_space.resize_with(config.max_boards, || {
            RwLock::new(Node::new(Board::new(
                0,
                0,
                config.max_width,
                config.max_height,
                config.max_snakes,
            )))
        });

        let mut thread_state = Vec::with_capacity(config.num_worker_threads);
        thread_state.resize_with(config.num_worker_threads, || {
            Mutex::new(ThreadContext {
                rng: R::new(),
                board: Board::new(0, 0, config.max_width, config.max_height, config.max_snakes),
                food_buff: Vec::with_capacity((config.max_width * config.max_height) as usize),

                play_scores: vec![0.0; config.max_snakes as usize],
            })
        });

        SearchContext {
            config: config.clone(),
            game: RwLock::new(None),

            search_lock: Mutex::new(()),
            node_space,
            thread_state,

            search_timeout: AtomicBool::new(false),
            done_barrier: Barrier::new(config.num_worker_threads + 1),
            total_nodes: AtomicI64::new(0),
            num_games: AtomicI64::new(0),
            num_playouts: AtomicI64::new(0),
        }
    }

    pub fn reset(&self) {
        self.total_nodes.store(0, Ordering::Release);
        self.num_games.store(0, Ordering::Release);
        self.num_playouts.store(0, Ordering::Release);
        self.search_timeout.store(false, Ordering::Release);
    }
}

impl Node {
    pub fn new(board: Board) -> Self {
        let max_snakes = board.max_snakes();
        Node {
            board,
            parent_node_idx: 0,
            parent_move_idx: 0,
            depth: 0,
            max_depth: 0,
            virtual_loss: false,
            games: 0,
            score: vec![0.0; max_snakes as usize],
            cache: vec![Default::default(); max_snakes as usize],
            num_children: 0,
            num_move_perms: 0,
            children: vec![NodePtr { moves: 0, index: 0 }; max_children(max_snakes)],
        }
    }

    pub fn reset(&mut self) {
        self.parent_move_idx = 0;
        self.parent_node_idx = 0;
        self.depth = 0;
        self.max_depth = 0;

        self.virtual_loss = false;
        self.games = 0;
        self.num_children = 0;
        self.num_move_perms = 0;

        for snake_score in self.score.iter_mut() {
            *snake_score = 0.0;
        }

        for score_cache in self.cache.iter_mut() {
            *score_cache = Default::default();
        }
    }

    pub fn child_moves(&self, idx: usize) -> u32 {
        self.children[idx].moves
    }

    pub fn max_children(&self) -> i32 {
        let num_alive_snakes = self.board.num_alive_snakes();
        Move::num_perm(num_alive_snakes) as i32
    }

    pub fn is_fully_expanded(&self) -> bool {
        self.num_move_perms as i32 >= self.max_children()
    }

    pub fn duct_score(&self, cfg: &Config, snake_idx: usize, mv: Move) -> f64 {
        let mcts_scores = &self.cache[snake_idx][mv.idx()];

        let virtual_loss = if self.virtual_loss { cfg.virtual_loss } else { 0.0 };

        if mcts_scores.pruned {
            f64::MIN
        } else if mcts_scores.games == 0 || self.games == 0 {
            if self.virtual_loss {
                -virtual_loss
            } else {
                f64::MAX
            }
        } else {
            let heuristic_mult = (cfg.equiv as f64 / (3.0 * self.games as f64 + cfg.equiv as f64)).sqrt();

            let uct_score = cfg.temperature * ((self.games as f64).ln() / mcts_scores.games as f64).sqrt();

            (1.0 - heuristic_mult) * (mcts_scores.score / mcts_scores.games as f64)
                + uct_score
                + heuristic_mult * mcts_scores.heuristic_score
                - virtual_loss
        }
    }
}

pub fn best_move(scores: &Scores, print_summary: bool) -> Move {
    let mut best_move_score = Move::Left;
    let mut best_move_games = Move::Left;
    let mut best_score = 0.0;
    let mut most_games = 0;

    let mut search_str = "".to_owned();

    for (mv_idx, stats) in scores.iter().enumerate() {
        let final_score = if stats.games == 0 {
            0.0
        } else {
            stats.score / stats.games as f64
        };

        search_str.push_str(&format!(
            "\nSearch move: {:<6} score: {:.8}  games: {}",
            Move::from_idx(mv_idx),
            final_score,
            stats.games
        ));

        if final_score > best_score {
            best_move_score = Move::from_idx(mv_idx);
            best_score = final_score;
        }

        if stats.games > most_games {
            best_move_games = Move::from_idx(mv_idx);
            most_games = stats.games;
        }
    }

    // Only use the game count if we are trapped
    let best_move = if best_score == 0.0 {
        best_move_games
    } else {
        best_move_score
    };

    if print_summary {
        search_str.push_str(&format!("\nSearch best move: {}", best_move));
        search_str.push_str(&format!("\nSearch best move score: {}", best_move_score));
        search_str.push_str(&format!("\nSearch best move games: {}", best_move_games));

        info!("{}", search_str);
    }
    best_move
}

pub fn search_moves<R: Rand>(
    ctx: Arc<SearchContext<R>>,
    pool: &ThreadPool,
    board: &Board,
    game: &Game,
    start_time: Instant,
) -> Result<SearchStats, Error> {
    let _search_guard = match ctx.search_lock.try_lock() {
        Ok(guard) => guard,
        Err(e) => return Err(Error::LockHeld(e.to_string())),
    };

    // Reset search state
    ctx.reset();
    {
        let mut game_guard = ctx.game.write().unwrap();
        *game_guard = Some(game.clone());
    }

    // Set the root node
    {
        let mut root_state_guard = ctx.node_space[0].write().unwrap();

        root_state_guard.reset();
        root_state_guard.board = board.clone();
    }

    ctx.total_nodes.fetch_add(1, Ordering::AcqRel);

    for id in 0..ctx.config.num_worker_threads {
        let ctx_cln = ctx.clone();
        pool.execute(move || search_worker(ctx_cln, id));
    }

    let startup_dur = Instant::now() - start_time;
    let search_us = (game.api.timeout - ctx.config.latency) as i64 * 1000;
    let search_dur = Duration::from_micros(search_us as u64).saturating_sub(startup_dur);

    if !search_dur.is_zero() {
        sleep(search_dur);
        info!(
            "Startup time {}us, Slept {}us",
            startup_dur.as_micros(),
            search_dur.as_micros()
        )
    } else {
        warn!("Search duration negative, no time to search");
    }

    ctx.search_timeout.store(true, Ordering::Release);
    ctx.done_barrier.wait();

    let root_guard = ctx.node_space[0].read().unwrap();

    let mut scores = Vec::with_capacity(ctx.config.max_snakes as usize);

    for s_idx in 0..root_guard.board.num_snakes() as usize {
        let mut snake_scores: Scores = Default::default();

        for (mv_idx, stats) in root_guard.cache[s_idx].iter().enumerate() {
            if !stats.pruned && stats.games != 0 {
                snake_scores[mv_idx].score = stats.score;
                snake_scores[mv_idx].games = stats.games;
            }
        }

        scores.push(snake_scores);
    }

    let max_depth = root_guard.max_depth;
    let total_nodes = ctx.total_nodes.load(Ordering::Relaxed);
    let num_games = ctx.num_games.load(Ordering::Relaxed);
    let num_playouts = ctx.num_playouts.load(Ordering::Relaxed);
    let num_terminal = num_games - num_playouts;

    info!("search max depth: {}", max_depth);
    info!("search total nodes: {}", total_nodes);
    info!("search num games: {}", num_games);
    info!("search num playouts: {}", num_playouts);
    info!("search num terminal: {}", num_terminal);

    Ok(SearchStats {
        total_nodes,
        max_depth,
        scores,
    })
}

fn search_worker<R: Rand>(ctx: Arc<SearchContext<R>>, id: usize) {
    let mut scratch_guard = ctx.thread_state[id].lock().unwrap();

    let game_guard = ctx.game.read().unwrap();
    let game = game_guard.as_ref().unwrap();

    let root_num_alive = { ctx.node_space[0].read().unwrap().board.num_alive_snakes() };

    'main_loop: loop {
        if ctx.search_timeout.load(Ordering::Acquire) {
            break 'main_loop;
        }

        // Find the next leaf node
        let mut curr_idx = 0;
        loop {
            {
                let mut state_guard = ctx.node_space[curr_idx].write().unwrap();

                if curr_idx != 0 {
                    state_guard.virtual_loss = true;
                }

                // We have reached a leaf that is end of the game
                // OR reached node that hasn't been played out yet.
                if game.over(&state_guard.board) || curr_idx != 0 && state_guard.games == 0 {
                    // Set the board for playout
                    scratch_guard.board = state_guard.board.clone();
                    break;
                }

                // Expand node if game not over
                if !state_guard.is_fully_expanded() {
                    let space_left = expand_node(
                        &ctx,
                        game,
                        &mut scratch_guard,
                        &mut state_guard,
                        curr_idx as u32,
                        root_num_alive,
                    );
                    if !space_left {
                        break 'main_loop;
                    }
                }
            }

            {
                let state_guard = ctx.node_space[curr_idx].read().unwrap();
                let mut max_score = f64::MIN;
                let mut max_idx = 0;

                // Assign DUCT scores to child nodes based on snake-move
                for child_ptr in state_guard.children[0..state_guard.num_children as usize].iter() {
                    let mut child_score = 0.0;

                    for snake_idx in 0..state_guard.board.num_snakes() {
                        let mv = Move::extract(child_ptr.moves, snake_idx as u32);
                        let mv_duct_score = state_guard.duct_score(&ctx.config, snake_idx as usize, mv);

                        child_score += mv_duct_score;
                    }

                    if child_score >= max_score {
                        max_score = child_score;
                        max_idx = child_ptr.index;
                    }
                }

                curr_idx = max_idx as usize;
            }
        }

        scratch_guard.play_scores.clear();

        // Perform rollout
        let is_terminal = playout_game(&ctx, &mut scratch_guard, game);

        if !is_terminal {
            ctx.num_playouts.fetch_add(1, Ordering::Relaxed);
        }
        ctx.num_games.fetch_add(1, Ordering::Relaxed);

        // Update rollout node score, get parent node for backprop
        let mut curr_move_idx;
        let new_depth;

        (curr_idx, curr_move_idx, new_depth) = {
            let mut state_guard = ctx.node_space[curr_idx].write().unwrap();

            for snake_idx in 0..state_guard.board.num_snakes() as usize {
                state_guard.score[snake_idx] += scratch_guard.play_scores[snake_idx];
            }

            state_guard.games += 1;
            state_guard.virtual_loss = false;

            (
                state_guard.parent_node_idx as usize,
                state_guard.parent_move_idx,
                state_guard.max_depth,
            )
        };

        // Backpropagate results from playout
        loop {
            let mut state_guard = ctx.node_space[curr_idx].write().unwrap();

            state_guard.games += 1;

            if new_depth > state_guard.max_depth {
                state_guard.max_depth = new_depth;
            }

            let moves = state_guard.children[curr_move_idx as usize].moves;

            for snake_idx in 0..state_guard.board.num_snakes() as usize {
                let snake_score = scratch_guard.play_scores[snake_idx];

                // Update cached score and game count of each snake-move for this node
                let snake_mv = Move::extract(moves, snake_idx as u32);

                let cache = &mut state_guard.cache[snake_idx][snake_mv.idx()];

                cache.score += snake_score;
                cache.games += 1;
            }

            if curr_idx == 0 {
                break;
            } else {
                state_guard.virtual_loss = false;
                curr_idx = state_guard.parent_node_idx as usize;
                curr_move_idx = state_guard.parent_move_idx;
            }
        }
    }
    ctx.done_barrier.wait();
}

fn playout_game<R: Rand>(_ctx: &SearchContext<R>, state: &mut ThreadContext<R>, game: &Game) -> bool {
    let mut terminal = true;
    let mut playout_moves: u32 = 0;

    while !game.over(&state.board) {
        terminal = false;

        for s in 0..state.board.num_snakes() as usize {
            if !state.board.snakes[s].alive() {
                continue;
            }

            playout_moves = Move::set_move(playout_moves, s as u32, state.board.gen_move(game, s, &mut state.rng));
        }

        state
            .board
            .gen_board(playout_moves, game, &mut state.food_buff, &mut state.rng);
    }

    for snake_idx in 0..state.board.num_snakes() as usize {
        let score = game.score(&state.board, snake_idx);
        state.play_scores.push(score);
    }
    terminal
}

fn expand_node<R: Rand>(
    ctx: &SearchContext<R>,
    game: &Game,
    state: &mut ThreadContext<R>,
    node: &mut Node,
    parent_index: u32,
    root_num_alive: i32,
) -> bool {
    let num_snakes = node.board.num_snakes() as usize;

    // Shouldn't try and expand an end of game board
    debug_assert!(!game.over(&node.board));

    let mut num_expanded = 0;

    // Iterate over permutations of moves for all alive snakes
    'expand_loop: while num_expanded < 1 && (node.num_move_perms as i32) < node.max_children() {
        let mut alive_index = 0;

        for s in 0..num_snakes {
            if !node.board.snakes[s].alive() {
                continue;
            }

            let snake_mv_idx = Move::extract_idx(node.num_move_perms, alive_index) as usize;
            let snake_move = Move::from_idx(snake_mv_idx);

            // If this is a bad move for the snake and it is not trapped, prune the node
            // Otherwise expand a single node for the snakes death
            if node.cache[s][snake_mv_idx].pruned {
                node.num_move_perms += 1;
                continue 'expand_loop;
            } else if (!node.board.valid_move(game, s, snake_move) && !node.board.is_trapped(game, s))
                || (node.board.is_trapped(game, s) && snake_move != Move::Left)
            {
                node.cache[s][snake_mv_idx].pruned = true;
                node.num_move_perms += 1;
                continue 'expand_loop;
            } else {
                let moves = node.children[node.num_children as usize].moves;
                node.children[node.num_children as usize].moves = Move::set_move(moves, s as u32, snake_move);
            }

            alive_index += 1;
        }

        node.num_move_perms += 1;

        let child_idx = ctx.total_nodes.fetch_add(1, Ordering::AcqRel) as usize;
        if child_idx >= ctx.node_space.len() {
            error!("No more boards in search space");
            return false;
        }

        node.children[node.num_children as usize].index = child_idx as u32;
        let child_moves = node.child_moves(node.num_children as usize);

        {
            let mut child_state_guard = ctx.node_space[child_idx].write().unwrap();

            child_state_guard.reset();

            child_state_guard.board.set(&node.board);
            child_state_guard
                .board
                .gen_board(child_moves, game, &mut state.food_buff, &mut state.rng);

            child_state_guard.depth = node.depth + 1;
            child_state_guard.max_depth = child_state_guard.depth;
            child_state_guard.parent_node_idx = parent_index;
            child_state_guard.parent_move_idx = node.num_children;

            for s_idx in 0..num_snakes {
                let snake_mv_idx =
                    Move::extract_idx(node.children[node.num_children as usize].moves, s_idx as u32) as usize;
                node.cache[s_idx][snake_mv_idx].heuristic_score +=
                    game.approx_score(&child_state_guard.board, &ctx.config, s_idx, root_num_alive)
            }
        }

        node.num_children += 1;
        num_expanded += 1;
    }
    true
}

pub fn max_children(max_snakes: i32) -> usize {
    // 1 extra space for royale mode shrink direction
    Move::num_perm(max_snakes + 1) as usize
}

#[cfg(test)]
mod search_test;
