use crate::search;
use crate::util;

use crate::api::{BattleState, IndexResp, MoveResp};
use crate::board::Board;
use crate::config::Config;
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::search::{Node, SearchContext};
use crate::util::Error;

use deepsize::DeepSizeOf;
use log::{error, info, warn};
use serde_json;
use tiny_http::{Method, Request, Response, StatusCode};

use std::{
    io::Cursor,
    sync::atomic::{AtomicBool, AtomicI64, Ordering},
    sync::{Arc, Barrier, Mutex},
    time::{Duration, Instant},
};

pub struct Server {
    server_pool: ThreadPool,
    state: Arc<ServerState>,
}

struct ServerState {
    config: Config,
    // initialization / termination
    done_flag: AtomicBool,
    ready_flag: AtomicBool,
    done_barrier: Barrier,

    // initialization / termination
    game_id: Mutex<Option<String>>,
    ticks: AtomicI64,
    search_pool: ThreadPool,
    context: Arc<SearchContext>,
    max_nodes: Mutex<i64>,
}

type StrResponse = Response<Cursor<Vec<u8>>>;

impl Server {
    const TICK_MS: u64 = 100;

    pub fn new(config: Config) -> Self {
        let config_clone = config.clone();

        Server {
            server_pool: ThreadPool::new(config.num_server_threads),

            state: Arc::new(ServerState {
                config,
                done_flag: AtomicBool::new(false),
                ready_flag: AtomicBool::new(false),
                done_barrier: Barrier::new(config_clone.num_server_threads + 1),
                game_id: Mutex::new(None),
                ticks: AtomicI64::new(0),
                search_pool: ThreadPool::new(config_clone.num_threads),
                context: Arc::new(SearchContext::new(config_clone)),
                max_nodes: Mutex::new(0),
            }),
        }
    }

    pub fn stop_server(&self) {
        info!("Stopping treesnake");
        self.state.done_flag.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.state.ready_flag.load(Ordering::Acquire)
    }

    pub fn wait_done(&self) {
        self.state.done_barrier.wait();
        info!("Exiting treesnake");
    }

    pub fn start_server(&self) {
        let test_node = Node::new(Board::new(
            0,
            0,
            self.state.config.max_width,
            self.state.config.max_height,
            self.state.config.max_snakes,
        ));

        let node_size = test_node.deep_size_of();
        let num_boards = self.state.config.max_boards;
        let space_size = node_size as i64 * num_boards as i64;

        info!("Size of Node: {}B", node_size);

        info!("Approx. size of search space: {}MiB", space_size >> 20);
        info!("Approx. size of search space: {}GiB", space_size >> 30);

        info!(
            "Starting search space allocation max_boards: {}, width: {}, height: {}, max_snakes {}",
            self.state.config.max_boards,
            self.state.config.max_width,
            self.state.config.max_height,
            self.state.config.max_snakes
        );

        self.state.context.allocate();

        info!("Allocation complete");

        let addr = format!("0.0.0.0:{}", self.state.config.port);
        let server = Arc::new(tiny_http::Server::new(tiny_http::ServerConfig { addr, ssl: None }).unwrap());

        info!("Started treesnake");
        self.state.ready_flag.store(true, Ordering::Release);

        for _ in 0..self.server_pool.num_threads() {
            let server = server.clone();
            let state = self.state.clone();
            let num_server_threads = self.server_pool.num_threads();

            self.server_pool.execute(move || {
                loop {
                    if state.done_flag.load(Ordering::Acquire) {
                        break;
                    }

                    // Blocks until the next request is received
                    let request_opt = match server.recv_timeout(Duration::from_millis(Self::TICK_MS)) {
                        Ok(rq) => rq,
                        Err(e) => {
                            error!("HTTP recv error: {}", e);
                            continue;
                        }
                    };

                    if request_opt.is_none() {
                        // Timeout games after ~10 seconds without hearing from that ID
                        let mut game_id_guard = state.game_id.lock().unwrap();
                        if game_id_guard.is_none() {
                            continue;
                        }

                        let ticks = state.ticks.fetch_add(1, Ordering::AcqRel);
                        if ticks > (100 * num_server_threads as i64) {
                            info!("TIMEOUT! Killing game ID {}", game_id_guard.as_ref().unwrap());
                            let mut game_guard = state.context.game.write().unwrap();
                            *game_id_guard = None;
                            *game_guard = None;
                            state.ticks.store(0, Ordering::Release);
                        }
                        continue;
                    }

                    info!(" --- start");

                    let mut request = request_opt.unwrap();

                    let start_time = Instant::now();

                    let method = request.method().clone();
                    let url = request.url()[1..].to_owned();

                    let response = get_response(state.clone(), &mut request, start_time);

                    let code = response.status_code().0;

                    request.respond(response).unwrap();

                    let dur = (start_time.elapsed().as_micros() as f64) / 1000.0;
                    let msg = format!(" --- end {} {} code {} duration {}ms\n\n", method, url, code, dur);

                    if code < 400 {
                        info!("{}", msg);
                    } else {
                        error!("{}", msg);
                    }
                }
                state.done_barrier.wait();
            });
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop_server();
        self.wait_done();
    }
}

fn get_response(state: Arc<ServerState>, request: &mut Request, start_time: Instant) -> StrResponse {
    match request.method() {
        Method::Get => match request.url() {
            "/" => StrResponse::from_string(
                serde_json::to_string(&IndexResp {
                    apiversion: "1".to_owned(),
                    author: "treesnake".to_owned(),
                    color: "#C42B3A".to_owned(),
                    head: "sand-worm".to_owned(),
                    tail: "fat-rattle".to_owned(),
                    version: ("1.0.0").to_owned(),
                })
                .unwrap(),
            ),
            _ => StrResponse::from_string("").with_status_code(StatusCode(404)),
        },
        Method::Post => {
            let mut content = String::new();
            request.as_reader().read_to_string(&mut content).unwrap();
            let parsed_req: Result<BattleState, serde_json::Error> = serde_json::from_str(&content);

            if parsed_req.is_err() {
                error!("Error parsing board state: - {}", Error::from(parsed_req.unwrap_err()));
                return StrResponse::from_string("{}").with_status_code(StatusCode(400));
            }
            let game_state = parsed_req.unwrap();
            let api_latency = game_state.you.latency.clone();
            let timeout = game_state.game.timeout;

            info!("Server reported latency: {} - timeout: {}", api_latency, timeout);
            if let Ok(l) = str::parse::<i32>(&api_latency) {
                if l >= timeout {
                    error!("Excessive reported latency: {}", l);
                }
            }

            let mut correct_game = false;
            {
                let game_id_guard = state.game_id.lock().unwrap();
                if game_id_guard.is_none() {
                    correct_game = true;
                    let mut game_guard = state.context.game.write().unwrap();

                    let is_solo = game_state.board.snakes.len() == 1;
                    let game_id = game_state.game.id.clone();

                    *game_guard = match Game::new(game_state.game.clone(), is_solo) {
                        Ok(game) => Some(game),
                        Err(e) => {
                            error!("Error parsing game {} {}", game_id, e);
                            return StrResponse::from_string("{}").with_status_code(StatusCode(400));
                        }
                    };
                } else if *game_id_guard.as_ref().unwrap() == game_state.game.id {
                    correct_game = true;
                    state.ticks.store(0, Ordering::Release);
                }
            }

            let game_width = game_state.board.width;
            let game_height = game_state.board.height;
            let game_snakes = game_state.board.snakes.len() as i32;

            match request.url() {
                "/start" => {
                    info!("game ID: {}", game_state.game.id);
                    info!("rules: {:?}", game_state.game.ruleset.name);
                    info!("map: {:?}", game_state.game.map);
                    info!("timeout: {}", game_state.game.timeout);
                    info!(
                        "width: {}, height: {}, snakes: {}",
                        game_width, game_height, game_snakes
                    );
                    info!(
                        "food spawn chance: {}",
                        game_state.game.ruleset.settings.food_spawn_chance
                    );
                    info!("min food: {}", game_state.game.ruleset.settings.minimum_food);
                    StrResponse::from_string("{}")
                }
                "/move" => {
                    // Find empty game slot
                    if !correct_game {
                        warn!("Game already running");
                        return StrResponse::from_string("{}").with_status_code(StatusCode(409));
                    } else if state.config.max_width < game_width
                        || state.config.max_height < game_height
                        || state.config.max_snakes < game_snakes
                    {
                        warn!(
                            "Server not configured for w: {} h: {} max_snakes: {}",
                            game_width, game_height, game_snakes
                        );
                        warn!(
                            "Current settings are max_width: {} max_height: {} max_snakes: {}",
                            state.config.max_width, state.config.max_height, state.config.max_snakes
                        );
                        return StrResponse::from_string("{}").with_status_code(StatusCode(409));
                    }

                    let ctx = state.context.clone();

                    info!("turn: {}", game_state.turn);

                    let parsed_board = Board::from_req(
                        game_state,
                        state.config.max_width,
                        state.config.max_height,
                        state.config.max_snakes,
                    );
                    let (mv, code) = match parsed_board {
                        Err(e) => {
                            error!("Error parsing board - {}", e);
                            (util::rand_move(), 400)
                        }
                        Ok(board) => {
                            info!("board:\n{}", board);
                            let latency = if api_latency.is_empty() {
                                0
                            } else {
                                api_latency.parse().expect("Error parsing server-reported latency")
                            };

                            {
                                let mut prev_boards_guard = ctx.game.write().unwrap();
                                prev_boards_guard.as_mut().unwrap().add_board(board)
                            }
                            let search_results = search::best_move(ctx, &state.search_pool, start_time, latency);
                            {
                                let mut max_nodes_guard = state.max_nodes.lock().unwrap();
                                if search_results.total_nodes > *max_nodes_guard {
                                    *max_nodes_guard = search_results.total_nodes;
                                }
                                info!("max nodes expanded: {}", *max_nodes_guard);
                            }

                            (search_results.best_move, 200)
                        }
                    };

                    let resp_str = serde_json::to_string(&MoveResp { mv }).unwrap();
                    StrResponse::from_string(resp_str).with_status_code(code)
                }
                "/end" => StrResponse::from_string("{}").with_status_code(StatusCode(200)),
                _ => StrResponse::from_string("{}").with_status_code(StatusCode(404)),
            }
        }
        _ => StrResponse::from_string("{}").with_status_code(StatusCode(404)),
    }
}
