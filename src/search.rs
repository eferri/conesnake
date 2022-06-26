use crate::board::Board;
use crate::config::Config;
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::util::{max_children, Coord, Move};

use log::{info, warn};

use std::sync::{atomic::AtomicBool, atomic::AtomicI64, atomic::Ordering, Arc, Barrier};
use std::sync::{Mutex, RwLock};
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
    num_games: AtomicI64,
    num_playouts: AtomicI64,
}

// Thread-local search resources, pre-allocated
pub struct ThreadContext {
    board: Board,
    playout_moves: Vec<Move>,
    food_buff: Vec<Coord>,

    play_scores: Vec<f64>,
    all_node_scores: Vec<SearchSort>,
}

#[derive(Clone)]
pub struct SearchSort {
    index: usize,
    total_score: f64,
}

pub struct SearchResult {
    pub best_move: Move,
    pub max_depth: i32,
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

    depth: i32,
    max_depth: i32,

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
            num_games: AtomicI64::new(0),
            num_playouts: AtomicI64::new(0),
        }
    }

    pub fn allocate(&self) {
        let mut space_guard = self.node_space.write().unwrap();

        space_guard.resize_with(self.config.max_boards, || {
            Node::new(Board::new(
                0,
                0,
                self.config.max_width,
                self.config.max_height,
                self.config.max_snakes,
            ))
        });

        let mut scratch_guard = self.thread_scratch.write().unwrap();
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
            })
        });
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

    fn duct_score(&self, snake_idx: usize, mv: Move, temp: f64) -> f64 {
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

pub fn best_move(ctx: Arc<SearchContext>, start_time: Instant, measured_latency: i32, slot: usize) -> SearchResult {
    let adaptive_search_us = {
        let mut game_guard = ctx.game.write().unwrap();
        game_guard.as_mut().unwrap().next_delay_us(measured_latency)
    };

    let game_guard = ctx.game.read().unwrap();
    let game = game_guard.as_ref().unwrap();

    // Reset search state
    ctx.reset();

    // Set the root node
    {
        let space_guard = ctx.node_space.read().unwrap();
        let mut root_state_guard = space_guard[0].state.write().unwrap();

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
        info!("Slot {} Startup time {}us, Slept {}us", slot, startup_us, search_us)
    } else {
        warn!(
            "Slot {} Search duration negative, no time to search: {}us",
            slot, search_us
        );
    }

    ctx.search_timeout.store(true, Ordering::Release);
    ctx.done_barrier.wait();

    let space_guard = ctx.node_space.read().unwrap();
    let root_state_guard = space_guard[0].state.read().unwrap();

    let mut best_move_score = Move::Left;
    let mut best_move_games = Move::Left;
    let mut best_score = 0.0;
    let mut most_games = 0;

    for (mv_idx, stats) in root_state_guard.cache[0].iter().enumerate() {
        if !stats.pruned && stats.games != 0 {
            let raw_score = stats.score / stats.games as f64;

            info!(
                "Slot {} Move {:?} score: {} games: {}",
                slot,
                Move::from_idx(mv_idx),
                raw_score,
                stats.games
            );

            if raw_score > best_score {
                best_move_score = Move::from_idx(mv_idx);
                best_score = raw_score;
            }

            if stats.games > most_games {
                best_move_games = Move::from_idx(mv_idx);
                most_games = stats.games;
            }
        }
    }

    // Only use the game count if we are trapped
    let best_move = if best_score == 0.0 {
        best_move_games
    } else {
        best_move_score
    };

    let max_depth = root_state_guard.max_depth;
    let total_nodes = ctx.total_nodes.load(Ordering::Relaxed);
    let num_games = ctx.num_games.load(Ordering::Relaxed);
    let num_playouts = ctx.num_playouts.load(Ordering::Relaxed);
    let num_terminal = num_games - num_playouts;

    info!("Slot {} search max depth: {}", slot, max_depth);
    info!("Slot {} search total nodes: {}", slot, total_nodes);
    info!("Slot {} search num games: {}", slot, num_games);
    info!("Slot {} search num playouts: {}", slot, num_playouts);
    info!("Slot {} search num terminal: {}", slot, num_terminal);

    SearchResult {
        best_move,
        max_depth,
        total_nodes,
    }
}

fn search_worker(ctx: Arc<SearchContext>, id: usize) {
    let space_guard = ctx.node_space.read().unwrap();
    let thread_guard = ctx.thread_scratch.read().unwrap();
    let mut scratch_guard = thread_guard[id].write().unwrap();

    let game_guard = ctx.game.read().unwrap();
    let game = game_guard.as_ref().unwrap();

    'main_loop: loop {
        if ctx.search_timeout.load(Ordering::Acquire) {
            break 'main_loop;
        }

        // Find the next leaf node
        let mut curr_idx = 0;
        let mut playout_guard = None;
        loop {
            let try_expand = {
                let state_guard = space_guard[curr_idx].state.read().unwrap();

                // We have reached a leaf that is end of the game
                // OR reached node that hasn't been played out yet.
                if game.over(&state_guard.board) || curr_idx != 0 && playout_guard.is_some() && state_guard.games == 0 {
                    break;
                } else {
                    playout_guard = None;
                }
                state_guard.num_children == 0
            };

            if try_expand {
                // Expand node if game not over, and another thread hasn't already expanded
                let mut state_guard = space_guard[curr_idx].state.write().unwrap();
                if state_guard.num_children == 0 {
                    expand_node(&ctx, &mut scratch_guard, &mut state_guard, curr_idx, game);
                }
            }

            let state_guard = space_guard[curr_idx].state.read().unwrap();

            scratch_guard.all_node_scores.clear();

            // Assign DUCT scores to child nodes based on snake-move
            for child_ptr in state_guard.children[0..state_guard.num_children].iter() {
                let mut node_totals = SearchSort {
                    index: child_ptr.index,
                    total_score: 0.0,
                };

                for (snake_idx, mv) in child_ptr.moves.iter().enumerate() {
                    let mv_duct_score = state_guard.duct_score(snake_idx, *mv, ctx.config.temperature);

                    node_totals.total_score += mv_duct_score;
                }

                scratch_guard.all_node_scores.push(node_totals.clone());
            }

            let sort_fn = |a: &SearchSort, b: &SearchSort| b.total_score.partial_cmp(&a.total_score).unwrap();

            scratch_guard.all_node_scores.sort_by(sort_fn);

            // Pick the child node with the highest DUCT score
            // or a child of the highest child node if all direct children are locked
            for score in scratch_guard.all_node_scores.iter() {
                let guard_opt = space_guard[score.index].play_lock.try_lock();
                if let Ok(guard) = guard_opt {
                    playout_guard = Some(guard);
                    curr_idx = score.index;
                    break;
                }
            }
            if playout_guard.is_none() {
                curr_idx = scratch_guard.all_node_scores.first().unwrap().index;
            }
        }

        // Perform rollout
        let is_terminal = {
            let _playout_guard = playout_guard;

            let (depth, board) = {
                let state_guard = space_guard[curr_idx].state.read().unwrap();
                (state_guard.depth, state_guard.board.clone())
            };

            scratch_guard.board = board;
            scratch_guard.play_scores.clear();
            playout_game(&ctx, &mut scratch_guard, game, depth)
        };

        if !is_terminal {
            ctx.num_playouts.fetch_add(1, Ordering::Relaxed);
        }
        ctx.num_games.fetch_add(1, Ordering::Relaxed);

        // Update rollout node score, get parent node for backprop
        let mut curr_move_idx;
        let new_depth;
        (curr_idx, curr_move_idx, new_depth) = {
            let mut state_guard = space_guard[curr_idx].state.write().unwrap();

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
            let mut parent_state_guard = space_guard[curr_idx].state.write().unwrap();

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

fn playout_game(_ctx: &SearchContext, state: &mut ThreadContext, game: &Game, depth: i32) -> bool {
    let mut terminal = true;

    while !game.over(&state.board) {
        terminal = false;

        for s in 0..state.board.num_snakes() as usize {
            if !state.board.snakes[s].alive {
                continue;
            }

            let snake_head = state.board.snakes[s].head;
            state.playout_moves[s] = state.board.rand_valid_move(snake_head, game.api.ruleset.name);
        }

        let moves = &state.playout_moves[0..state.board.num_snakes() as usize];
        state.board = state.board.gen_board(moves, game, &mut state.food_buff);
    }

    for snake_idx in 0..state.board.num_snakes() as usize {
        let score = game.score(&state.board, snake_idx, depth);
        state.play_scores.push(score);
    }
    terminal
}

fn expand_node(ctx: &SearchContext, state: &mut ThreadContext, node: &mut NodeState, parent_index: usize, game: &Game) {
    let num_snakes = node.board.num_snakes() as usize;
    let num_alive_snakes = node.board.num_alive_snakes();
    let space_guard = ctx.node_space.read().unwrap();

    let rules = game.api.ruleset.name;

    // Shouldn't try and expand an end of game board
    debug_assert!(!game.over(&node.board));

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
            let mut child_state_guard = space_guard[child_idx].state.write().unwrap();

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
