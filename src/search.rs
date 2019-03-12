use crate::board::Board;
use crate::config::Config;
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::util::{max_children, Coord, Move};

use log::{info, warn};
use parking_lot::{Mutex, RwLock};

use std::sync::{atomic::AtomicBool, atomic::AtomicI64, atomic::Ordering, Arc, Barrier};
use std::{thread::sleep, time::Duration, time::Instant};

// Global search resources shared among search threads
pub struct SearchContext {
    pub config: Config,
    pub game: RwLock<Option<Game>>,

    search_pool: ThreadPool,
    node_space: RwLock<Vec<Node>>,
    thread_scratch: RwLock<Vec<RwLock<ThreadContext>>>,

    // Search state
    search_timeout: AtomicBool,
    done_barrier: Barrier,
    total_nodes: AtomicI64,
}

// Thread-local search resources, pre-allocated
pub struct ThreadContext {
    board: Board,
    playout_moves: Vec<Move>,
    food_buff: Vec<Coord>,

    play_scores: Vec<f64>,
    unlocked_node_scores: Vec<SearchSort>,
    all_node_scores: Vec<SearchSort>,
}

#[derive(Clone)]
pub struct SearchSort {
    index: usize,
    total_score: f64,
}

pub struct SearchResult {
    pub best_move: Move,
    pub max_depth: i64,
    pub total_nodes: i64,
}

// Node in search tree
pub struct Node {
    play_lock: Mutex<()>,
    state: RwLock<NodeState>,
}

#[derive(Clone)]
pub struct NodeState {
    board: Board,

    parent_node_idx: usize,
    parent_move_idx: usize,

    depth: i64,
    max_depth: i64,

    score: Vec<f64>,
    games: i64,
    cache: Vec<[NodeScoreCache; 4]>,

    num_children: usize,
    children: Vec<NodePtr>,
}

#[derive(Default, Debug, Clone)]
pub struct NodeScoreCache {
    score: f64,
    games: i64,
    pruned: bool,
}

// Pointer to child nodes, with corresponding moves
#[derive(Clone)]
pub struct NodePtr {
    moves: Vec<Move>,
    index: usize,
}

impl SearchContext {
    pub fn new(config: Config) -> Self {
        let num_cpu = config.num_threads;
        SearchContext {
            config,

            game: RwLock::new(None),
            search_pool: ThreadPool::new(num_cpu),
            node_space: RwLock::new(Vec::new()),
            thread_scratch: RwLock::new(Vec::new()),

            search_timeout: AtomicBool::new(false),
            done_barrier: Barrier::new(num_cpu + 1),
            total_nodes: AtomicI64::new(0),
        }
    }

    pub fn allocate(&self) {
        let mut space_guard = self.node_space.write();

        space_guard.resize_with(self.config.max_boards, || {
            Node::new(Board::new(
                0,
                0,
                self.config.max_width,
                self.config.max_height,
                self.config.max_snakes,
            ))
        });

        let mut scratch_guard = self.thread_scratch.write();
        scratch_guard.resize_with(self.config.num_threads, || {
            RwLock::new(ThreadContext {
                board: Board::new(
                    0,
                    0,
                    self.config.max_width,
                    self.config.max_height,
                    self.config.max_snakes,
                ),
                playout_moves: vec![Move::Left; self.config.max_snakes as usize],
                food_buff: Vec::with_capacity((self.config.max_width * self.config.max_height) as usize),

                play_scores: vec![0.; self.config.max_snakes as usize],
                all_node_scores: Vec::with_capacity(max_children(self.config.max_snakes)),
                unlocked_node_scores: Vec::with_capacity(max_children(self.config.max_snakes)),
            })
        });
    }

    pub fn reset(&self) {
        self.total_nodes.store(0, Ordering::Release);
        self.search_timeout.store(false, Ordering::Release);
    }
}

impl Node {
    pub fn new(board: Board) -> Self {
        let max_snakes = board.max_snakes();
        Node {
            play_lock: Mutex::new(()),

            state: RwLock::new(NodeState {
                board,

                parent_node_idx: 0,
                parent_move_idx: 0,

                depth: 0,
                max_depth: 0,

                games: 0,
                score: vec![0.0; max_snakes as usize],
                cache: vec![Default::default(); max_snakes as usize],

                num_children: 0,
                children: vec![
                    NodePtr {
                        moves: Vec::with_capacity(max_snakes as usize),
                        index: 0,
                    };
                    max_children(max_snakes)
                ],
            }),
        }
    }
}

impl NodeState {
    pub fn reset(&mut self) {
        self.parent_move_idx = 0;
        self.parent_node_idx = 0;
        self.depth = 0;
        self.max_depth = 0;
        self.num_children = 0;

        self.games = 0;

        for snake_score in self.score.iter_mut() {
            *snake_score = 0.0;
        }

        for score_cache in self.cache.iter_mut() {
            *score_cache = Default::default();
        }
    }

    pub fn child_moves(&self, idx: usize) -> &[Move] {
        &self.children[idx].moves
    }

    fn duct_score(&self, snake_idx: usize, mv: Move) -> f64 {
        let temp = 1.41;
        let cache = &self.cache[snake_idx][mv.idx()];

        if cache.pruned {
            0.0
        } else if cache.games == 0 || self.games == 0 {
            f64::INFINITY
        } else {
            cache.score / cache.games as f64 + temp * ((self.games as f64).ln() / cache.games as f64).sqrt()
        }
    }
}

pub fn best_move(ctx: Arc<SearchContext>, start_time: Instant, measured_latency: i32) -> SearchResult {
    let adaptive_search_us = {
        let mut game_guard = ctx.game.write();
        game_guard.as_mut().unwrap().next_delay_us(measured_latency)
    };

    let game_guard = ctx.game.read();
    let game = game_guard.as_ref().unwrap();

    // Reset search state
    ctx.reset();

    // Set the root node
    {
        let space_guard = ctx.node_space.read();
        let mut root_state_guard = space_guard[0].state.write();

        root_state_guard.reset();
        root_state_guard.board = game.start_board().clone();
    }

    ctx.total_nodes.fetch_add(1, Ordering::AcqRel);

    for id in 0..ctx.config.num_threads {
        let ctx_cln = ctx.clone();
        ctx.search_pool.execute(move || search_worker(ctx_cln, id));
    }

    let startup_us = (Instant::now() - start_time).as_micros() as i64;
    let search_us = adaptive_search_us - startup_us;

    if ctx.config.always_sleep {
        sleep(Duration::from_micros(adaptive_search_us as u64));
    }
    if search_us > 0 {
        sleep(Duration::from_micros(search_us as u64));
        info!("Startup time {}us, Slept {}us", startup_us, search_us)
    } else {
        warn!("Search duration negative, no time to search: {}us", search_us);
    }

    ctx.search_timeout.store(true, Ordering::Release);
    ctx.done_barrier.wait();

    let space_guard = ctx.node_space.read();
    let root_state_guard = space_guard[0].state.read();

    let mut best_move = Move::Left;
    let mut best_score = 0.0;
    for (mv_idx, stats) in root_state_guard.cache[0].iter().enumerate() {
        if !stats.pruned && stats.games != 0 {
            let raw_score = stats.score / stats.games as f64;

            info!(
                "Move {:?} score: {} games: {}",
                Move::from_idx(mv_idx),
                raw_score,
                stats.games
            );

            if raw_score > best_score {
                best_move = Move::from_idx(mv_idx);
                best_score = raw_score;
            }
        }
    }

    let max_depth = root_state_guard.max_depth;
    let total_nodes = ctx.total_nodes.load(Ordering::Relaxed);

    info!("Search max depth: {}", max_depth);
    info!("Search total nodes: {}", total_nodes);

    SearchResult {
        best_move,
        max_depth,
        total_nodes,
    }
}

fn search_worker(ctx: Arc<SearchContext>, id: usize) {
    let space_guard = ctx.node_space.read();
    let thread_guard = ctx.thread_scratch.read();
    let mut scratch_guard = thread_guard[id].write();

    let game_guard = ctx.game.read();
    let game = game_guard.as_ref().unwrap();

    let mut parent_idx = 0;
    let mut reset_parent = false;

    'main_loop: loop {
        // Find the next leaf node
        let mut curr_idx = 0;
        if reset_parent {
            curr_idx = parent_idx;
        }
        reset_parent = false;

        loop {
            if ctx.search_timeout.load(Ordering::Acquire) {
                break 'main_loop;
            }

            {
                let state_guard = space_guard[curr_idx].state.read();
                let node_locked = space_guard[curr_idx].play_lock.try_lock().is_none();

                // We have reached a leaf that is end of the game
                // OR reached node that hasn't been played out yet.
                if game.over(&state_guard.board)
                    || state_guard.depth > ctx.config.max_depth
                    || curr_idx != 0 && !node_locked && state_guard.games == 0
                {
                    break;
                }
            }

            let try_expand = space_guard[curr_idx].state.read().num_children == 0;

            if try_expand {
                // Expand node if game not over, and another thread hasn't already expanded
                let mut state_guard = space_guard[curr_idx].state.write();
                if state_guard.num_children == 0 {
                    expand_node(&ctx, &mut scratch_guard, &mut state_guard, curr_idx, game);
                }
            }

            let state_guard = space_guard[curr_idx].state.read();

            scratch_guard.unlocked_node_scores.clear();
            scratch_guard.all_node_scores.clear();

            // Assign DUCT scores to child nodes based on snake-move
            for child_ptr in state_guard.children[0..state_guard.num_children].iter() {
                // Another thread is already playing a game on this child node, ignore it
                let child_locked = space_guard[child_ptr.index].play_lock.try_lock().is_none();

                let mut node_totals = SearchSort {
                    index: child_ptr.index,
                    total_score: 0.0,
                };

                for (snake_idx, mv) in child_ptr.moves.iter().enumerate() {
                    let mv_duct_score = state_guard.duct_score(snake_idx, *mv);

                    node_totals.total_score += mv_duct_score;
                }

                scratch_guard.all_node_scores.push(node_totals.clone());
                if !child_locked {
                    scratch_guard.unlocked_node_scores.push(node_totals)
                }
            }

            let sort_fn = |a: &SearchSort, b: &SearchSort| a.total_score.partial_cmp(&b.total_score).unwrap();

            scratch_guard.all_node_scores.sort_by(sort_fn);
            scratch_guard.unlocked_node_scores.sort_by(sort_fn);

            // Pick the non-locked child node with the highest total DUCT score, or a random child, if none
            parent_idx = curr_idx;
            if scratch_guard.unlocked_node_scores.is_empty() {
                curr_idx = scratch_guard.all_node_scores.last().unwrap().index;
            } else {
                curr_idx = scratch_guard.unlocked_node_scores.last().unwrap().index;
            }
        }

        // Try to lock the chosen node for rollout.
        // If another thread got the node, go back to parent node and chose again
        {
            let playout_res = space_guard[curr_idx].play_lock.try_lock();
            if playout_res.is_none() {
                reset_parent = true;
                continue 'main_loop;
            }

            let depth = {
                let start_state_guard = space_guard[curr_idx].state.read();
                scratch_guard.board = start_state_guard.board.clone();
                start_state_guard.depth
            };

            scratch_guard.play_scores.clear();
            playout_game(&ctx, &mut scratch_guard, game, depth);
        }

        // Update rollout node score, get parent node for backprop
        let mut curr_move_idx;
        let new_depth;
        (curr_idx, curr_move_idx, new_depth) = {
            let mut state_guard = space_guard[curr_idx].state.write();

            for snake_idx in 0..state_guard.board.num_snakes() as usize {
                state_guard.score[snake_idx] += scratch_guard.play_scores[snake_idx];
            }
            state_guard.games += 1;
            (
                state_guard.parent_node_idx,
                state_guard.parent_move_idx,
                state_guard.max_depth,
            )
        };

        // Backpropagate results from playout
        loop {
            let mut parent_state_guard = space_guard[curr_idx].state.write();

            parent_state_guard.games += 1;

            if new_depth > parent_state_guard.max_depth {
                parent_state_guard.max_depth = new_depth;
            }

            for snake_idx in 0..parent_state_guard.board.num_snakes() as usize {
                let snake_score = scratch_guard.play_scores[snake_idx];

                // Update cached score and game count of each snake-move for this node
                let snake_mv = parent_state_guard.children[curr_move_idx].moves[snake_idx];
                let mut cache = &mut parent_state_guard.cache[snake_idx][snake_mv.idx()];

                cache.score += snake_score;
                cache.games += 1;
            }

            if curr_idx == 0 {
                break;
            } else {
                curr_idx = parent_state_guard.parent_node_idx;
                curr_move_idx = parent_state_guard.parent_move_idx;
            }
        }
    }
    ctx.done_barrier.wait();
}

fn playout_game(ctx: &SearchContext, state: &mut ThreadContext, game: &Game, depth: i64) {
    let mut curr_depth = depth;

    let score_snakes = |state: &mut ThreadContext, curr_depth: i64| {
        for snake_idx in 0..state.board.num_snakes() as usize {
            let score = game.score(&state.board, snake_idx, curr_depth, ctx.config.max_depth);
            state.play_scores.push(score);
        }
    };

    if game.over(&state.board) || depth > ctx.config.max_depth {
        score_snakes(state, curr_depth);
        return;
    }

    while !game.over(&state.board) && curr_depth <= ctx.config.max_depth {
        if ctx.search_timeout.load(Ordering::Acquire) {
            score_snakes(state, curr_depth);
            return;
        }

        for s in 0..state.board.num_snakes() as usize {
            if !state.board.snakes[s].alive {
                continue;
            }

            let snake_head = state.board.snakes[s].head;
            state.playout_moves[s] = state.board.rand_valid_move(snake_head, game.api.ruleset.name);
        }

        let moves = &state.playout_moves[0..state.board.num_snakes() as usize];
        state.board = state.board.gen_board(moves, game, &mut state.food_buff);
        curr_depth += 1;
    }

    score_snakes(state, curr_depth);
}

fn expand_node(ctx: &SearchContext, state: &mut ThreadContext, node: &mut NodeState, parent_index: usize, game: &Game) {
    let num_snakes = node.board.num_snakes() as usize;
    let num_alive_snakes = node.board.num_alive_snakes();
    let space_guard = ctx.node_space.read();

    let rules = game.api.ruleset.name;

    // Shouldn't try and expand an end of game board
    debug_assert!(!game.over(&node.board) && node.depth <= ctx.config.max_depth);

    let num_alive_snake_moves = Move::num_move_perm(num_alive_snakes as usize);

    // Iterate over permutations of moves for all alive snakes
    'expand_loop: for move_idx in 0..num_alive_snake_moves {
        if node.children[node.num_children].moves.len() != num_snakes {
            node.children[node.num_children].moves.resize(num_snakes, Move::Left);
        }

        let mut alive_index = 0;

        for s in 0..num_snakes {
            if !node.board.snakes[s].alive {
                continue;
            }

            let snake_head = node.board.snakes[s].head;
            let snake_mv_idx = Move::get_perm_idx(move_idx, alive_index);
            let snake_move = Move::from_idx(snake_mv_idx);

            // If this is a bad move for the snake and it is not trapped, prune the node
            // Otherwise expand a single node for the snakes death
            if node.cache[s][snake_mv_idx].pruned {
                continue 'expand_loop;
            } else if (!node.board.valid_move(snake_head, snake_move, rules)
                && !node.board.is_trapped(snake_head, rules))
                || (node.board.is_trapped(snake_head, rules) && snake_move != Move::Left)
            {
                node.cache[s][snake_mv_idx].pruned = true;
                continue 'expand_loop;
            } else {
                node.children[node.num_children].moves[s] = snake_move;
            }

            alive_index += 1;
        }

        let child_idx = ctx.total_nodes.fetch_add(1, Ordering::AcqRel) as usize;
        node.children[node.num_children].index = child_idx;

        let child_moves = node.child_moves(node.num_children);

        {
            if child_idx >= space_guard.len() {
                panic!("No more boards in search space")
            }
            let mut child_state_guard = space_guard[child_idx].state.write();

            child_state_guard.reset();

            child_state_guard.board = node.board.gen_board(child_moves, game, &mut state.food_buff);

            child_state_guard.depth = node.depth + 1;
            child_state_guard.max_depth = child_state_guard.depth;
            child_state_guard.parent_node_idx = parent_index;
            child_state_guard.parent_move_idx = node.num_children;
        }

        node.num_children += 1;
    }
}

#[cfg(test)]
mod search_test;
