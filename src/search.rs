use crate::api::{Scores, SearchStats};
use crate::board::Board;
use crate::config::Config;
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::rand::Rand;
use crate::util::{Coord, Error, Move};

use deepsize::DeepSizeOf;
use log::{error, info, warn};

#[cfg(feature = "simd")]
use std::simd::{f64x4, mask64x4, num::SimdFloat, StdFloat};

use std::sync::{atomic::AtomicBool, atomic::AtomicI64, atomic::Ordering, Arc, Barrier};
use std::sync::{Mutex, RwLock};
use std::{thread::sleep, time::Duration, time::Instant};

// Global search resources shared among search threads
pub struct SearchContext<R: Rand> {
    pub config: Config,
    pub game: RwLock<Option<Game>>,

    search_lock: Mutex<()>,
    node_space: Vec<Node>,
    thread_state: Vec<Mutex<ThreadContext<R>>>,

    // Search state
    search_timeout: AtomicBool,
    out_of_space: AtomicBool,
    done_barrier: Barrier,
    total_nodes: AtomicI64,
    num_searches: AtomicI64,
    num_playouts: AtomicI64,
    playout_ns: AtomicI64,
}

// Thread-local search resources, pre-allocated
pub struct ThreadContext<R: Rand> {
    pub board: Board,

    rng: R,

    food_buff: Vec<Coord>,
    play_scores: Vec<f64>,
    child_scores: Vec<SearchSort>,
}

#[derive(Default, Debug, Copy, Clone)]
struct SearchSort {
    index: usize,
    total_score: f64,
}

// Node in search tree
#[derive(Clone, DeepSizeOf)]
pub struct NodeState {
    board: Board,

    parent_node_idx: u32,
    parent_move_idx: u32,

    depth: i32,
    max_depth: i32,

    games: i32,
    score: Vec<f64>,

    cache: Vec<[NodeScoreCache; 4]>,

    num_children: u32,
    num_move_perms: u32,
    // This uses a LOT of memory
    // each node has 4^max_snakes children
    children: Vec<NodePtr>,
}

#[derive(DeepSizeOf)]
pub struct Node {
    playout_lock: Mutex<()>,
    state: RwLock<NodeState>,
}

#[derive(Default, Debug, Clone, DeepSizeOf)]
struct NodeScoreCache {
    score: f64,
    games: i64,
    variance_sum: f64,
    pruned: bool,
}

// Pointer to child nodes, with corresponding moves
// 2-bit moves are encoded into "moves" u32 to save memory
// In Royale map, shrink direction is encoded in last 2 bits
#[derive(Clone, DeepSizeOf)]
struct NodePtr {
    moves: u32,
    index: u32,
}

impl<R: Rand> ThreadContext<R> {
    pub fn new(cfg: &Config) -> Self {
        ThreadContext {
            rng: R::new(),
            board: Board::new(0, 0, cfg.max_width, cfg.max_height, cfg.max_snakes),
            food_buff: Vec::with_capacity((cfg.max_width * cfg.max_height) as usize),

            play_scores: Vec::with_capacity(cfg.max_snakes as usize),
            child_scores: Vec::with_capacity(max_children(cfg.max_snakes)),
        }
    }
}

impl<R: Rand> SearchContext<R> {
    pub fn new(config: &Config) -> Self {
        let mut node_space = Vec::with_capacity(config.max_boards);
        node_space.resize_with(config.max_boards, || {
            Node::new(Board::new(0, 0, config.max_width, config.max_height, config.max_snakes))
        });

        let mut thread_state = Vec::with_capacity(config.num_threads);
        thread_state.resize_with(config.num_threads, || Mutex::new(ThreadContext::new(config)));

        SearchContext {
            config: config.clone(),
            game: RwLock::new(None),

            search_lock: Mutex::new(()),
            node_space,
            thread_state,

            search_timeout: AtomicBool::new(false),
            out_of_space: AtomicBool::new(false),
            done_barrier: Barrier::new(config.num_threads + 1),
            total_nodes: AtomicI64::new(0),
            num_searches: AtomicI64::new(0),
            num_playouts: AtomicI64::new(0),
            playout_ns: AtomicI64::new(0),
        }
    }

    pub fn reset(&self) {
        self.total_nodes.store(0, Ordering::Release);
        self.num_searches.store(0, Ordering::Release);
        self.num_playouts.store(0, Ordering::Release);
        self.search_timeout.store(false, Ordering::Release);
        self.playout_ns.store(0, Ordering::Release);
    }
}

impl Node {
    pub fn new(board: Board) -> Self {
        Node {
            playout_lock: Mutex::new(()),
            state: RwLock::new(NodeState::new(board)),
        }
    }
}

impl NodeState {
    pub fn new(board: Board) -> Self {
        let max_snakes = board.max_snakes();
        NodeState {
            board,
            parent_node_idx: 0,
            parent_move_idx: 0,
            depth: 0,
            max_depth: 0,
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

        if mcts_scores.games < cfg.min_playouts || (self.games as i64) < cfg.min_playouts {
            f64::MAX
        } else {
            let ln_parent_games = (self.games as f64).ln();

            let variance = if mcts_scores.games < 2 {
                0.0
            } else {
                (mcts_scores.variance_sum) / (mcts_scores.games - 1) as f64
            };

            let var_ucb = variance + (2.0 * ln_parent_games / mcts_scores.games as f64).sqrt();
            let uct_score = ((0.25_f64.min(var_ucb) * ln_parent_games) / mcts_scores.games as f64).sqrt();

            (mcts_scores.score / mcts_scores.games as f64) + cfg.temperature * uct_score
        }
    }

    #[cfg(feature = "simd")]
    pub fn duct_scores_simd(&self, cfg: &Config, mvs: u32) -> f64x4 {
        let mut op_mask = mask64x4::splat(false);

        let mut results = f64x4::splat(0.0);
        let mut variance_sums = f64x4::splat(0.0);
        let mut games = f64x4::splat(0.0);
        let mut scores = f64x4::splat(0.0);

        for snake_idx in 0..self.board.num_snakes() as usize {
            let mv = Move::extract(mvs, snake_idx as u32);
            let mcts_scores = &self.cache[snake_idx][mv.idx()];

            if mcts_scores.games < cfg.min_playouts || (self.games as i64) < cfg.min_playouts {
                results[snake_idx] = f64::MAX
            } else {
                op_mask.set(snake_idx, true);
                variance_sums[snake_idx] = mcts_scores.variance_sum as f64;
                games[snake_idx] = mcts_scores.games as f64;
                scores[snake_idx] = mcts_scores.score as f64;
            }
        }

        if !op_mask.any() {
            return results;
        }

        let ln_parent_games = (self.games as f64).ln();

        let variance_mask = op_mask & games.gt(&f64x4::splat(1.0));

        let variances = variance_mask.select(variance_sums / (games - f64x4::splat(1.0)), f64x4::splat(0.0));
        let var_ucb = op_mask.select(
            variances + (f64x4::splat(2.0 * ln_parent_games) / games).sqrt(),
            f64x4::splat(0.0),
        );

        let uct_score = op_mask.select(
            ((f64x4::splat(0.25).simd_min(var_ucb) * f64x4::splat(ln_parent_games)) / games).sqrt(),
            f64x4::splat(0.0),
        );

        op_mask.select((scores / games) + f64x4::splat(cfg.temperature) * uct_score, results)
    }
}

pub fn best_move(cfg: &Config, snake_idx: usize, scores: &[Scores], print_summary: bool) -> Move {
    let mut best_move_score = Move::Left;
    let mut best_move_games = Move::Left;
    let mut best_score = cfg.loss_val;
    let mut most_games = 0;

    let mut search_str = "".to_owned();

    for (mv_idx, stats) in scores[snake_idx].iter().enumerate() {
        let final_score = if stats.games == 0 {
            cfg.loss_val
        } else {
            stats.score / stats.games as f64
        };

        let mv = Move::from_idx(mv_idx);

        search_str.push_str(&format!(
            "\nSearch move: {:<6} score: {:.8}  games: {}",
            mv, final_score, stats.games
        ));

        if final_score > best_score {
            best_move_score = mv;
            best_score = final_score;
        }

        if stats.games > most_games {
            best_move_games = mv;
            most_games = stats.games;
        }
    }

    // Only use the game count if we are trapped
    let best_move = if best_score == cfg.loss_val {
        best_move_games
    } else {
        best_move_score
    };

    if print_summary {
        search_str.push_str(&format!("\nSearch best move: {best_move}"));
        search_str.push_str(&format!("\nSearch best move score: {best_move_score}"));
        search_str.push_str(&format!("\nSearch best move games: {best_move_games}"));

        info!("{}", search_str);
    }
    best_move
}

pub fn mcts<R: Rand>(
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
        let mut root_state_guard = ctx.node_space[0].state.write().unwrap();

        root_state_guard.reset();
        root_state_guard.board.set_from(board);
    }

    ctx.total_nodes.fetch_add(1, Ordering::AcqRel);

    for id in 0..ctx.config.num_threads {
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

    let root_guard = ctx.node_space[0].state.read().unwrap();

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
    let total_nodes = ctx.total_nodes.load(Ordering::Acquire);
    let num_searches = ctx.num_searches.load(Ordering::Acquire);
    let num_playouts = ctx.num_playouts.load(Ordering::Acquire);
    let num_terminal = num_searches - num_playouts;
    let playout_ns = ctx.playout_ns.load(Ordering::Acquire);
    let avg_playout_us = (playout_ns as f64 / num_playouts as f64) / (1000.0);

    info!("search max depth: {}", max_depth);
    info!("search total nodes: {}", total_nodes);
    info!("search num games: {}", num_searches);
    info!("search num playouts: {}", num_playouts);
    info!("search num terminal: {}", num_terminal);
    info!("Average playout us: {}", avg_playout_us);

    Ok(SearchStats {
        total_nodes,
        num_playouts,
        num_searches,
        max_depth,
        scores,
    })
}

fn search_worker<R: Rand>(ctx: Arc<SearchContext<R>>, id: usize) {
    let mut scratch_guard = ctx.thread_state[id].lock().unwrap();

    let game_guard = ctx.game.read().unwrap();
    let game = game_guard.as_ref().unwrap();

    'main_loop: loop {
        if ctx.search_timeout.load(Ordering::Acquire) {
            break 'main_loop;
        }

        // Select leaf node for playout
        let mut curr_idx = 0;
        let mut playout_guard = None;

        loop {
            {
                let mut state_guard = ctx.node_space[curr_idx].state.write().unwrap();

                // We have reached a leaf that is end of the game
                // OR reached node that hasn't been played out yet.
                if game.over(&state_guard.board) || curr_idx != 0 && playout_guard.is_some() && state_guard.games == 0 {
                    // Set the board for playout
                    scratch_guard.board.set_from(&state_guard.board);
                    break;
                } else {
                    playout_guard = None;
                }

                // Expand node if game not over
                if !state_guard.is_fully_expanded() {
                    let space_left = expand_node(&ctx, game, &mut scratch_guard, &mut state_guard, curr_idx as u32);
                    if !space_left {
                        ctx.out_of_space.store(true, Ordering::Release);
                        break 'main_loop;
                    }
                }
            }

            {
                let state_guard = ctx.node_space[curr_idx].state.read().unwrap();
                if ctx.out_of_space.load(Ordering::Acquire) {
                    break 'main_loop;
                }

                scratch_guard.child_scores.clear();

                // Assign DUCT scores to child nodes based on snake-move
                for child_ptr in state_guard.children[0..state_guard.num_children as usize].iter() {
                    let mut node_totals = SearchSort {
                        index: child_ptr.index as usize,
                        total_score: 0.0,
                    };

                    #[cfg(not(feature = "simd"))]
                    for snake_idx in 0..state_guard.board.num_snakes() {
                        let mv = Move::extract(child_ptr.moves, snake_idx as u32);
                        let mv_duct_score = state_guard.duct_score(&ctx.config, snake_idx as usize, mv);

                        node_totals.total_score += mv_duct_score;
                    }

                    #[cfg(feature = "simd")]
                    {
                        let duct_scores = state_guard.duct_scores_simd(&ctx.config, child_ptr.moves);

                        node_totals.total_score += duct_scores.reduce_sum() as f64;
                    }

                    scratch_guard.child_scores.push(node_totals);
                }

                let sort_fn = |a: &SearchSort, b: &SearchSort| b.total_score.partial_cmp(&a.total_score).unwrap();

                scratch_guard.child_scores.sort_unstable_by(sort_fn);

                // Pick the child node with the highest DUCT score that isn't being played by another thread
                // or a child of the highest child node if all direct children are being played by other threads
                for score in scratch_guard.child_scores.iter() {
                    let guard_opt = ctx.node_space[score.index].playout_lock.try_lock();
                    if let Ok(guard) = guard_opt {
                        playout_guard = Some(guard);
                        curr_idx = score.index;
                        break;
                    }
                }
                if playout_guard.is_none() {
                    curr_idx = scratch_guard.child_scores.first().unwrap().index;
                }
            }
        }

        // Perform rollout
        let start_time = Instant::now();
        let (is_terminal, _) = {
            let _playout_guard = playout_guard;
            playout_game(&ctx.config, &mut scratch_guard, game)
        };
        let dur_ns = (Instant::now() - start_time).as_nanos() as i64;

        if !is_terminal {
            ctx.num_playouts.fetch_add(1, Ordering::Relaxed);
            ctx.playout_ns.fetch_add(dur_ns, Ordering::Relaxed);
        }
        ctx.num_searches.fetch_add(1, Ordering::Relaxed);

        // Update rollout node score, get parent node for backpropagation
        let mut curr_move_idx;
        let new_depth;

        (curr_idx, curr_move_idx, new_depth) = {
            let mut state_guard = ctx.node_space[curr_idx].state.write().unwrap();

            for snake_idx in 0..state_guard.board.num_snakes() as usize {
                state_guard.score[snake_idx] += scratch_guard.play_scores[snake_idx];
            }

            state_guard.games += 1;

            (
                state_guard.parent_node_idx as usize,
                state_guard.parent_move_idx,
                state_guard.max_depth,
            )
        };

        // Backpropagate results from playout
        loop {
            let mut state_guard = ctx.node_space[curr_idx].state.write().unwrap();

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

                let old_score = cache.score;
                let old_games = cache.games;

                cache.score += snake_score;
                cache.games += 1;

                if old_games > 0 {
                    let old_mean = old_score / old_games as f64;
                    let new_mean = cache.score / cache.games as f64;
                    cache.variance_sum += (snake_score - old_mean) * (snake_score - new_mean);
                }
            }

            if curr_idx == 0 {
                break;
            } else {
                curr_idx = state_guard.parent_node_idx as usize;
                curr_move_idx = state_guard.parent_move_idx;
            }
        }
    }
    ctx.done_barrier.wait();
}

pub fn playout_game<R: Rand>(cfg: &Config, state: &mut ThreadContext<R>, game: &Game) -> (bool, i32) {
    let mut terminal = true;
    let mut playout_moves: u32 = 0;
    let mut num_moves = 0;

    state.play_scores.clear();

    while !game.over(&state.board) {
        terminal = false;

        for s in 0..state.board.num_snakes() as usize {
            if !state.board.snakes[s].alive() {
                continue;
            }

            let mv = if cfg.strong_playout {
                state.board.gen_strong_move(game, s, &mut state.rng)
            } else {
                state.board.gen_move(game, s, &mut state.rng)
            };

            playout_moves = Move::set_move(playout_moves, s as u32, mv);
        }

        state
            .board
            .gen_board(playout_moves, game, &mut state.food_buff, &mut state.rng);

        num_moves += 1;
    }

    for snake_idx in 0..state.board.num_snakes() as usize {
        let score = game.score(&state.board, cfg, snake_idx);
        state.play_scores.push(score);
    }
    (terminal, num_moves)
}

fn expand_node<R: Rand>(
    ctx: &SearchContext<R>,
    game: &Game,
    state: &mut ThreadContext<R>,
    node: &mut NodeState,
    parent_index: u32,
) -> bool {
    let num_snakes = node.board.num_snakes() as usize;

    // Shouldn't try and expand an end of game board
    debug_assert!(!game.over(&node.board));

    let mut num_expanded = 0;
    let num_to_expand = if node.depth == 0 { node.max_children() } else { 1 };

    // Iterate over permutations of moves for all alive snakes
    'expand_loop: while num_expanded < num_to_expand && (node.num_move_perms as i32) < node.max_children() {
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
            let mut child_state_guard = ctx.node_space[child_idx].state.write().unwrap();

            child_state_guard.reset();

            child_state_guard.board.set_from(&node.board);
            child_state_guard
                .board
                .gen_board(child_moves, game, &mut state.food_buff, &mut state.rng);

            child_state_guard.depth = node.depth + 1;
            child_state_guard.max_depth = child_state_guard.depth;
            child_state_guard.parent_node_idx = parent_index;
            child_state_guard.parent_move_idx = node.num_children;
        }

        node.num_children += 1;
        num_expanded += 1;
    }
    true
}

pub fn max_children(max_snakes: i32) -> usize {
    Move::num_perm(max_snakes) as usize
}

#[cfg(test)]
mod search_test;
