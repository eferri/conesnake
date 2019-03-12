use crate::search;
use crate::util;

use crate::api::{BattleState, IndexResp, MoveResp};
use crate::board::Board;
use crate::config::Config;
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::search::SearchContext;
use crate::util::Error;

use log::{error, info, warn};
use parking_lot::RwLock;
use serde_json;
use tiny_http::{Method, Request, Response, StatusCode};

use std::{
    fs,
    io::Cursor,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    sync::Barrier,
    time::Duration,
    time::Instant,
};

pub struct Server {
    num_req_workers: usize,
    req_pool: ThreadPool,

    state: Arc<ServerState>,
    context: Arc<SearchContext>,
}

struct ServerState {
    pub config: Config,
    pub game_id: RwLock<Option<String>>,
    pub done_flag: AtomicBool,
    pub ready_flag: AtomicBool,
    pub done_barrier: Barrier,
}

type StrResponse = Response<Cursor<Vec<u8>>>;

impl Server {
    pub fn new(config: Config) -> Self {
        let num_threads = config.num_threads;
        Server {
            num_req_workers: num_threads,
            req_pool: ThreadPool::new(num_threads),

            state: Arc::new(ServerState {
                config: config.clone(),
                game_id: RwLock::new(None),
                done_flag: AtomicBool::new(false),
                ready_flag: AtomicBool::new(false),
                done_barrier: Barrier::new(num_threads + 1),
            }),

            context: Arc::new(SearchContext::new(config)),
        }
    }
    pub fn stop_server(&self) {
        info!("Snake server exiting");
        self.state.done_flag.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.state.ready_flag.load(Ordering::Acquire)
    }

    pub fn wait_done(&self) {
        self.state.done_barrier.wait();
    }

    pub fn start_server(&self) {
        info!(
            "Starting search space allocation max_boards: {}, width: {}, height: {}, max_snakes {}",
            self.state.config.max_boards,
            self.state.config.max_width,
            self.state.config.max_height,
            self.state.config.max_snakes
        );

        self.context.allocate();

        info!("Allocation complete");

        let addr = format!("0.0.0.0:{}", self.state.config.port);

        let ssl = match (&self.state.config.certificate, &self.state.config.private_key) {
            (Some(c), Some(k)) => Some(tiny_http::SslConfig {
                certificate: fs::read(c).expect("Error reading certificate file"),
                private_key: fs::read(k).expect("Error reading private key file"),
            }),
            (None, None) => None,
            (None, Some(_)) | (Some(_), None) => panic!("Missing SSL certificate or private_key"),
        };

        let server = Arc::new(tiny_http::Server::new(tiny_http::ServerConfig { addr, ssl }).unwrap());

        info!("Started treesnake");
        self.state.ready_flag.store(true, Ordering::Release);

        for _ in 0..self.num_req_workers {
            let server = server.clone();
            let state = self.state.clone();
            let ctx = self.context.clone();

            self.req_pool.execute(move || {
                loop {
                    if state.done_flag.load(Ordering::Acquire) {
                        break;
                    }

                    // Blocks until the next request is received
                    let request_opt = match server.recv_timeout(Duration::from_millis(100)) {
                        Ok(rq) => rq,
                        Err(e) => {
                            error!("HTTP recv error: {}", e);
                            continue;
                        }
                    };
                    let start_time = Instant::now();

                    let mut request = match request_opt {
                        Some(req) => req,
                        None => {
                            continue;
                        }
                    };

                    let method = request.method().clone();
                    let url = request.url()[1..].to_owned();

                    let response = get_response(state.clone(), ctx.clone(), &mut request, start_time);

                    let code = response.status_code().0;

                    request.respond(response).unwrap();

                    let dur = (start_time.elapsed().as_micros() as f64) / 1000.0;
                    let msg = format!("{} {} code {} duration {}ms", method, url, code, dur);

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

fn get_response(
    state: Arc<ServerState>,
    ctx: Arc<SearchContext>,
    request: &mut Request,
    start_time: Instant,
) -> StrResponse {
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

            let latency = game_state.you.latency.clone();

            info!("Server reported latency: {}", latency);
            if let Ok(l) = str::parse::<i32>(&latency) {
                if l >= 500 {
                    error!("Excessive latency {}", l);
                }
            }

            match request.url() {
                "/start" => {
                    let game_width = game_state.board.width;
                    let game_height = game_state.board.height;
                    let game_snakes = game_state.board.snakes.len() as i32;

                    info!("game ID: {}", game_state.game.id);
                    info!("rules: {:?}", game_state.game.ruleset.name);
                    info!("map: {:?}", game_state.game.map);
                    info!("timeout: {}", game_state.game.timeout);
                    info!(
                        "food spawn chance: {}",
                        game_state.game.ruleset.settings.food_spawn_chance
                    );
                    info!("min food: {}", game_state.game.ruleset.settings.minimum_food);

                    if state.config.max_width < game_width
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

                    let has_game = { state.game_id.read().is_some() };
                    if has_game {
                        warn!("Incorrect game ID");
                        return StrResponse::from_string("{}").with_status_code(StatusCode(409));
                    } else {
                        let mut game_id_guard = state.game_id.write();
                        let mut game_guard = ctx.game.write();

                        *game_id_guard = Some(game_state.game.id.clone());

                        let is_solo = game_state.board.snakes.len() == 1;

                        *game_guard = match Game::from_req(
                            game_state.game,
                            state.config.fallback_latency,
                            state.config.latency_safety,
                            is_solo,
                        ) {
                            Ok(game) => Some(game),
                            Err(e) => {
                                error!("Error parsing game {}", e);
                                None
                            }
                        };
                    }

                    StrResponse::from_string("{}")
                }
                "/move" => {
                    let correct_game = {
                        let game_id_guard = state.game_id.read();
                        game_id_guard.is_some() && game_id_guard.as_ref().unwrap() == &game_state.game.id
                    };
                    if !correct_game {
                        warn!("Incorrect game ID");
                        return StrResponse::from_string("{}").with_status_code(StatusCode(409));
                    }

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
                            {
                                let mut game_guard = ctx.game.write();
                                game_guard.as_mut().unwrap().add_board(board);
                            }

                            let measured_latency = if latency.is_empty() {
                                0
                            } else {
                                latency.parse().expect("Error parsing server-reported latency")
                            };

                            (search::best_move(ctx, start_time, measured_latency).best_move, 200)
                        }
                    };

                    let resp_str = serde_json::to_string(&MoveResp { mv }).unwrap();
                    StrResponse::from_string(resp_str).with_status_code(code)
                }
                "/end" => {
                    let correct_game = {
                        let game_guard = ctx.game.read();
                        game_guard.is_some() && game_guard.as_ref().unwrap().api.id == game_state.game.id
                    };
                    if !correct_game {
                        warn!("Incorrect game ID");
                        return StrResponse::from_string("{}").with_status_code(StatusCode(409));
                    } else {
                        *ctx.game.write() = None;
                        *state.game_id.write() = None;
                    }
                    StrResponse::from_string("{}")
                }
                _ => StrResponse::from_string("").with_status_code(StatusCode(404)),
            }
        }
        _ => StrResponse::from_string("").with_status_code(StatusCode(404)),
    }
}
