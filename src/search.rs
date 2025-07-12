use crate::api::{Scores, SearchStats};
use crate::board::Board;
use crate::config::{Config, MAX_BOARD_SIZE, MAX_SNAKES};
use crate::game::Game;
use crate::game::Rules;
use crate::pool::ThreadPool;
use crate::rand::Rand;
use crate::util::{Coord, Error, Move};

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
    node_space: Vec<RwLock<Node>>,
    thread_state: Vec<Mutex<ThreadContext<R>>>,

    // Search state
    search_timeout: AtomicBool,
    out_of_space: AtomicBool,
    done_barrier: Barrier,
    total_nodes: AtomicI64,
    num_searches: AtomicI64,
    total_playouts: AtomicI64,
    total_turns: AtomicI64,
    playout_ns: AtomicI64,
}

// Thread-local search resources, pre-allocated
pub struct ThreadContext<R: Rand> {
    pub board: Board,

    rng: R,

    food_buff: [Coord; MAX_BOARD_SIZE],
    play_scores: [f64; MAX_SNAKES],
}

impl<R: Rand> Default for ThreadContext<R> {
    fn default() -> Self {
        ThreadContext::new()
    }
}

// Node in search tree
#[derive(Clone)]
pub struct Node {
    board: Board,

    parent_node_idx: u32,
    parent_move_idx: u32,

    depth: i32,
    max_depth: i32,

    games: i32,
    score: [f64; MAX_SNAKES],

    cache: [[NodeScoreCache; 4]; MAX_SNAKES],

    num_children: i32,
    num_move_perms: i32,
    child_idx: u32,

    // This uses a LOT of memory
    child_moves: [u16; Move::num_perm(MAX_SNAKES as i32) as usize],
}

#[derive(Default, Debug, Clone, Copy)]
struct NodeScoreCache {
    score: f64,
    games: i64,
    pruned: bool,
}

impl<R: Rand> ThreadContext<R> {
    pub fn new() -> Self {
        ThreadContext {
            rng: R::new(),
            board: Board::new(0, 0),
            food_buff: [Coord::new(0, 0); MAX_BOARD_SIZE],

            play_scores: [Default::default(); MAX_SNAKES],
        }
    }
}

impl<R: Rand> SearchContext<R> {
    pub fn new(config: &Config) -> Self {
        let mut node_space = Vec::with_capacity(config.max_boards);
        node_space.resize_with(config.max_boards, || RwLock::new(Node::new(Board::new(0, 0))));

        let mut thread_state = Vec::with_capacity(config.num_threads);
        thread_state.resize_with(config.num_threads, || Mutex::new(ThreadContext::new()));

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
            total_playouts: AtomicI64::new(0),
            total_turns: AtomicI64::new(0),
            playout_ns: AtomicI64::new(0),
        }
    }

    pub fn reset(&self) {
        self.total_nodes.store(0, Ordering::Release);
        self.num_searches.store(0, Ordering::Release);
        self.total_playouts.store(0, Ordering::Release);
        self.search_timeout.store(false, Ordering::Release);
        self.playout_ns.store(0, Ordering::Release);
    }
}

impl Node {
    pub fn new(board: Board) -> Self {
        Node {
            board,
            parent_node_idx: 0,
            parent_move_idx: 0,
            depth: 0,
            max_depth: 0,
            games: 0,
            score: [0.0; MAX_SNAKES],
            cache: [[Default::default(); 4]; MAX_SNAKES],
            num_children: 0,
            num_move_perms: 0,
            child_idx: 0,
            child_moves: [0; Move::num_perm(MAX_SNAKES as i32) as usize],
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
        self.child_idx = 0;

        for snake_score in self.score.iter_mut() {
            *snake_score = 0.0;
        }

        for score_cache in self.cache.iter_mut() {
            *score_cache = Default::default();
        }
    }

    pub fn max_children(&self) -> i32 {
        let num_alive_snakes = self.board.num_alive_snakes();
        Move::num_perm(num_alive_snakes) as i32
    }

    pub fn is_fully_expanded(&self) -> bool {
        self.num_move_perms as i32 >= self.max_children()
    }

    pub fn temperature(&self, cfg: &Config, game: &Game) -> f64 {
        if game.ruleset == Rules::Constrictor {
            cfg.temperature_constrictor
        } else {
            cfg.temperature
        }
    }

    pub fn duct_score(&self, cfg: &Config, game: &Game, snake_idx: usize, mv: Move) -> f64 {
        let mcts_scores = &self.cache[snake_idx][mv.idx()];

        if mcts_scores.games == 0 || (self.games as i64) == 0 {
            f64::MAX
        } else {
            let uct_score = self.temperature(cfg, game) * ((self.games as f64).ln() / mcts_scores.games as f64).sqrt();
            (mcts_scores.score / mcts_scores.games as f64) + uct_score
        }
    }

    #[cfg(feature = "simd")]
    pub fn duct_scores_simd(&self, cfg: &Config, game: &Game, mvs: u16) -> f64x4 {
        let mut op_mask = mask64x4::splat(false);

        let mut results = f64x4::splat(0.0);
        let mut games = f64x4::splat(0.0);
        let mut scores = f64x4::splat(0.0);

        for snake_idx in 0..self.board.num_snakes() as usize {
            if !self.board.snakes[snake_idx].alive() {
                results[snake_idx] = 0.0;
                continue;
            }

            let mv = Move::extract(mvs, snake_idx as u32);
            let mcts_scores = &self.cache[snake_idx][mv.idx()];

            if mcts_scores.games == 0 || (self.games as i64) == 0 {
                results[snake_idx] = f64::MAX
            } else {
                op_mask.set(snake_idx, true);
                games[snake_idx] = mcts_scores.games as f64;
                scores[snake_idx] = mcts_scores.score;
            }
        }

        if !op_mask.any() {
            return results;
        }

        let uct_score = op_mask.select(
            f64x4::splat(self.temperature(cfg, game)) * (f64x4::splat(self.games as f64).ln() / games).sqrt(),
            f64x4::splat(0.0),
        );

        op_mask.select((scores / games) + uct_score, results)
    }

    pub fn duct_score_wrapper(&self, cfg: &Config, game: &Game, moves: u16) -> f64 {
        #[cfg(not(feature = "simd"))]
        {
            let mut score = 0.0;
            for snake_idx in 0..self.board.num_snakes() {
                if !self.board.snakes[snake_idx].alive() {
                    results[snake_idx] = 0.0;
                    continue;
                }

                let mv = Move::extract(moves, snake_idx as u32);
                let mv_duct_score = self.duct_score(cfg, game, snake_idx as usize, mv);

                score += mv_duct_score;
            }
            return score;
        }

        #[cfg(feature = "simd")]
        {
            let duct_scores = self.duct_scores_simd(cfg, game, moves);

            duct_scores.reduce_sum()
        }
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

    let best_move = best_move_games;

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
        let mut root_node_guard = ctx.node_space[0].write().unwrap();

        root_node_guard.reset();
        root_node_guard.board.set_from(board);
    }

    ctx.total_nodes.fetch_add(1, Ordering::AcqRel);

    for id in 0..ctx.config.num_threads {
        let ctx_cln = ctx.clone();
        pool.execute(move || search_worker(ctx_cln, id));
    }

    if ctx.config.fixed_iter == 0 {
        let startup_dur = Instant::now() - start_time;
        let search_us = (game.api.timeout - ctx.config.latency) as i64 * 1000;
        let search_dur = Duration::from_micros(search_us as u64).saturating_sub(startup_dur);

        if !search_dur.is_zero() {
            sleep(search_dur);
            info!(
                "Startup time {}ns, Slept {}us",
                startup_dur.as_nanos(),
                search_dur.as_micros()
            )
        } else {
            warn!("Search duration negative, no time to search");
        }

        ctx.search_timeout.store(true, Ordering::Release);
    }

    ctx.done_barrier.wait();

    let root_guard = ctx.node_space[0].read().unwrap();

    let mut scores = [Default::default(); MAX_SNAKES];
    let num_snakes = root_guard.board.num_alive_snakes();

    for (s_idx, score) in scores.iter_mut().enumerate().take(num_snakes as usize) {
        let mut snake_scores: Scores = Default::default();

        for (mv_idx, stats) in root_guard.cache[s_idx].iter().enumerate() {
            if !stats.pruned && stats.games != 0 {
                snake_scores[mv_idx].score = stats.score;
                snake_scores[mv_idx].games = stats.games;
            }
        }

        *score = snake_scores;
    }

    let max_depth = root_guard.max_depth;
    let total_nodes = ctx.total_nodes.load(Ordering::Acquire);
    let num_searches = ctx.num_searches.load(Ordering::Acquire);
    let total_playouts = ctx.total_playouts.load(Ordering::Acquire);
    let total_turns = ctx.total_turns.load(Ordering::Acquire);
    let num_terminal = num_searches - total_playouts;
    let playout_ns = ctx.playout_ns.load(Ordering::Acquire);
    let avg_playout_ns = playout_ns as f64 / total_playouts as f64;
    let avg_turn_ns = playout_ns as f64 / total_turns as f64;

    let stats = SearchStats {
        total_nodes,
        num_searches,
        total_playouts,
        total_turns,
        num_terminal,
        avg_playout_ns,
        avg_turn_ns,
        max_depth,
        num_snakes,
        scores,
    };

    if ctx.config.fixed_iter == 0 {
        info!("search:\n{}", stats);
    }

    Ok(stats)
}

fn search_worker<R: Rand>(ctx: Arc<SearchContext<R>>, id: usize) {
    let mut scratch_guard = ctx.thread_state[id].lock().unwrap();

    let game_guard = ctx.game.read().unwrap();
    let game = game_guard.as_ref().unwrap();

    'main_loop: loop {
        if ctx.search_timeout.load(Ordering::Acquire)
            || (ctx.config.fixed_iter > 0 && ctx.num_searches.load(Ordering::Acquire) >= ctx.config.fixed_iter)
        {
            break 'main_loop;
        }

        // Select leaf node for playout
        let mut curr_idx = 0;

        loop {
            if ctx.out_of_space.load(Ordering::Acquire) {
                break 'main_loop;
            }

            let node_guard = ctx.node_space[curr_idx].read().unwrap();

            // Leaf node: we have reached a node that is end of the game
            // OR reached node with a child that hasn't been played out yet.
            if game.over(&node_guard.board) || !node_guard.is_fully_expanded() {
                // Still set the board for playout (evaluation) if we lost
                scratch_guard.board.set_from(&node_guard.board);
                break;
            }

            // Assign DUCT scores to child nodes based on snake-move
            let mut max_score = f64::MIN;
            let mut max_idx_opt = None;

            for (idx, child_moves) in node_guard.child_moves[0..node_guard.num_children as usize]
                .iter()
                .enumerate()
            {
                let duct_score = node_guard.duct_score_wrapper(&ctx.config, game, *child_moves);

                if duct_score > max_score || max_idx_opt.is_none() {
                    max_score = duct_score;
                    max_idx_opt = Some(node_guard.child_idx + idx as u32)
                }
            }

            debug_assert!(max_idx_opt.is_some());

            curr_idx = max_idx_opt.unwrap() as usize;
        }

        // Expand the node
        if !game.over(&scratch_guard.board) {
            let mut node_guard = ctx.node_space[curr_idx].write().unwrap();

            let expand_res = expand_node(&ctx, game, &mut scratch_guard, &mut node_guard, curr_idx);
            match expand_res {
                Ok(expanded) => {
                    // The parent node is write-locked, meaning no other thread can be reading this node
                    // This means no other thread could have access to this node for playout
                    if expanded {
                        // If a new node was created, play a game from that one
                        curr_idx = (node_guard.child_idx as i32 + node_guard.num_children) as usize;
                    } else {
                        // otherwise the node is now fully expanded. Go back to choose one of its children
                        // Go back to choose one of its children
                        continue 'main_loop;
                    }
                }
                Err(e) => {
                    error!("Error expanding node: {}", e);
                    ctx.out_of_space.store(true, Ordering::Release);
                    break 'main_loop;
                }
            }
        }

        // Perform rollout
        let start_time = Instant::now();
        let mut is_terminal = false;
        let mut num_total_turns = 0;
        let num_playouts = 1;

        for _ in 0..num_playouts {
            let (is_curr_terminal, num_turns) = playout_game(&ctx.config, &mut scratch_guard, game);
            is_terminal |= is_curr_terminal;
            num_total_turns += num_turns;
        }
        let dur_ns = (Instant::now() - start_time).as_nanos() as i64;

        if !is_terminal {
            ctx.total_playouts.fetch_add(num_playouts, Ordering::Relaxed);
            ctx.playout_ns.fetch_add(dur_ns, Ordering::Relaxed);
        }
        ctx.total_turns.fetch_add(num_total_turns as i64, Ordering::Relaxed);
        ctx.num_searches.fetch_add(1, Ordering::AcqRel);

        // Update rollout node score, get parent node for backpropagation
        let mut curr_move_idx;
        let new_depth;

        (curr_idx, curr_move_idx, new_depth) = {
            let mut node_guard = ctx.node_space[curr_idx].write().unwrap();

            for snake_idx in 0..node_guard.board.num_snakes() as usize {
                node_guard.score[snake_idx] += scratch_guard.play_scores[snake_idx];
            }

            node_guard.games += 1;

            (
                node_guard.parent_node_idx as usize,
                node_guard.parent_move_idx,
                node_guard.max_depth,
            )
        };

        // Backpropagate results from playout
        loop {
            let mut node_guard = ctx.node_space[curr_idx].write().unwrap();

            node_guard.games += 1;

            if new_depth > node_guard.max_depth {
                node_guard.max_depth = new_depth;
            }

            let moves = node_guard.child_moves[curr_move_idx as usize];

            for snake_idx in 0..node_guard.board.num_snakes() as usize {
                let snake_score = scratch_guard.play_scores[snake_idx];

                // Update cached score and game count of each snake-move for this node
                let snake_mv = Move::extract(moves, snake_idx as u32);

                let cache = &mut node_guard.cache[snake_idx][snake_mv.idx()];

                cache.score += snake_score;
                cache.games += 1;
            }

            if curr_idx == 0 {
                break;
            } else {
                curr_idx = node_guard.parent_node_idx as usize;
                curr_move_idx = node_guard.parent_move_idx;
            }
        }
    }
    ctx.done_barrier.wait();
}

pub fn playout_game<R: Rand>(cfg: &Config, state: &mut ThreadContext<R>, game: &Game) -> (bool, i32) {
    let mut terminal = true;
    let mut playout_moves: u16 = 0;
    let mut num_moves = 0;

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
        state.play_scores[snake_idx] = score;
    }
    (terminal, num_moves)
}

fn expand_node<R: Rand>(
    ctx: &SearchContext<R>,
    game: &Game,
    state: &mut ThreadContext<R>,
    node: &mut Node,
    parent_index: usize,
) -> Result<bool, Error> {
    let num_snakes = node.board.num_snakes() as usize;

    // Shouldn't try and expand an end of game board
    debug_assert!(!game.over(&node.board));

    let mut expanded = false;

    'expand_loop: while node.num_move_perms < node.max_children() {
        let mut alive_index = 0;

        for s in 0..num_snakes {
            if !node.board.snakes[s].alive() {
                continue;
            }

            let snake_mv_idx = Move::extract_idx(node.num_move_perms as u16, alive_index) as usize;
            let snake_move = Move::from_idx(snake_mv_idx);

            // If this node is pruned, continue
            // Else if this is a bad move for the snake and it is not trapped,
            //  Or it is trapped but not a left move, prune the node.
            //  Expand a single node (left move) for the snake's death
            // Else expand the node
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
                let moves = node.child_moves[node.num_children as usize];
                node.child_moves[node.num_children as usize] = Move::set_move(moves, s as u32, snake_move);
            }

            alive_index += 1;
        }

        if node.num_children == 0 {
            node.child_idx = ctx.total_nodes.fetch_add(node.max_children() as i64, Ordering::AcqRel) as u32;
            if node.child_idx as usize >= ctx.node_space.len() {
                return Err(Error::ResourceError("No more boards in search space".to_owned()));
            }
        }

        expanded = true;
        node.num_children += 1;
        node.num_move_perms += 1;

        // Expand the nodes
        let child_moves = node.child_moves[node.num_children as usize];

        let mut child_node_guard = ctx.node_space[(node.child_idx as i32 + node.num_children) as usize]
            .write()
            .unwrap();

        child_node_guard.reset();

        child_node_guard.board.set_from(&node.board);
        child_node_guard
            .board
            .gen_board(child_moves, game, &mut state.food_buff, &mut state.rng);

        child_node_guard.depth = node.depth + 1;
        child_node_guard.max_depth = child_node_guard.depth;
        child_node_guard.parent_node_idx = parent_index as u32;
        child_node_guard.parent_move_idx = node.num_children as u32;
    }

    Ok(expanded)
}

#[cfg(test)]
mod search_test;
