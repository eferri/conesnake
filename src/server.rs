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
use parking_lot::Mutex;
use serde_json;
use tiny_http::{Method, Request, Response, StatusCode};

use std::{
    fs,
    io::Cursor,
    sync::atomic::{AtomicBool, AtomicI64, Ordering},
    sync::Arc,
    sync::Barrier,
    time::Duration,
    time::Instant,
};

pub struct Server {
    num_req_workers: usize,
    req_pool: ThreadPool,
    state: Arc<ServerState>,
}

struct ServerState {
    config: Config,
    done_flag: AtomicBool,
    ready_flag: AtomicBool,
    done_barrier: Barrier,
    game_slots: Vec<ServerGame>,
}

struct ServerGame {
    game_id: Mutex<Option<String>>,
    ticks: AtomicI64,
    context: Arc<SearchContext>,
}

type StrResponse = Response<Cursor<Vec<u8>>>;

impl Server {
    const TICK_MS: u64 = 100;

    pub fn new(config: Config) -> Self {
        // Extra threads to reject requests when all game slots full
        let num_req_threads = config.num_requests + 2;

        let mut slots = Vec::with_capacity(config.num_requests);
        for _ in 0..config.num_requests {
            slots.push(ServerGame {
                game_id: Mutex::new(None),
                ticks: AtomicI64::new(0),
                context: Arc::new(SearchContext::new(config.clone())),
            })
        }

        Server {
            num_req_workers: num_req_threads,
            req_pool: ThreadPool::new(num_req_threads),

            state: Arc::new(ServerState {
                config,
                done_flag: AtomicBool::new(false),
                ready_flag: AtomicBool::new(false),
                done_barrier: Barrier::new(num_req_threads + 1),
                game_slots: slots,
            }),
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

        for slot in &self.state.game_slots {
            slot.context.allocate();
        }

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
            let num_workers = self.num_req_workers;

            self.req_pool.execute(move || {
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
                        for slot in &state.game_slots {
                            let mut game_id_guard = slot.game_id.lock();
                            if game_id_guard.is_none() {
                                continue;
                            }

                            let ticks = slot.ticks.fetch_add(1, Ordering::AcqRel);
                            if ticks > (100 * num_workers as i64) {
                                info!("TIMEOUT! Killing game ID {}", game_id_guard.as_ref().unwrap());
                                let mut game_guard = slot.context.game.write();
                                *game_id_guard = None;
                                *game_guard = None;
                                slot.ticks.store(0, Ordering::Release);
                            }
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
            let latency = game_state.you.latency.clone();
            let timeout = game_state.game.timeout;

            info!("Server reported latency: {} - timeout: {}", latency, timeout);
            if let Ok(l) = str::parse::<i32>(&latency) {
                if l >= timeout {
                    error!("Excessive reported latency: {}", l);
                }
            }

            let mut slot_idx = None;
            for (idx, slot) in state.game_slots.iter().enumerate() {
                let game_id_guard = slot.game_id.lock();
                if game_id_guard.is_some() && *game_id_guard.as_ref().unwrap() == game_state.game.id {
                    slot_idx = Some(idx);
                    slot.ticks.store(0, Ordering::Release);
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

                    // Find empty game slot
                    let mut slot_found = false;
                    for (idx, slot) in state.game_slots.iter().enumerate() {
                        let mut id_guard = slot.game_id.lock();
                        if id_guard.is_some() {
                            continue;
                        }

                        let mut game_guard = slot.context.game.write();
                        let game_id = game_state.game.id.clone();

                        let is_solo = game_state.board.snakes.len() == 1;

                        *game_guard = match Game::from_req(
                            game_state.game,
                            state.config.fallback_latency,
                            state.config.latency_safety,
                            is_solo,
                        ) {
                            Ok(game) => Some(game),
                            Err(e) => {
                                error!("Error parsing game {} {}", game_id, e);
                                return StrResponse::from_string("{}").with_status_code(StatusCode(400));
                            }
                        };

                        info!("Game ID {} Slot {}", game_id, idx);

                        *id_guard = Some(game_id);
                        slot_found = true;
                        break;
                    }

                    if !slot_found {
                        warn!("No game slot found");
                        StrResponse::from_string("{}").with_status_code(StatusCode(400))
                    } else {
                        StrResponse::from_string("{}")
                    }
                }
                "/move" => {
                    if slot_idx.is_none() {
                        warn!("Game ID {} not found", game_state.game.id);
                        return StrResponse::from_string("{}").with_status_code(StatusCode(400));
                    }

                    let slot = slot_idx.unwrap();

                    let ctx = state.game_slots[slot].context.clone();

                    info!("Slot {} turn: {}", slot, game_state.turn);

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
                            info!("Slot {} board:\n{}", slot, board);
                            {
                                let mut game_guard = ctx.game.write();
                                game_guard.as_mut().unwrap().add_board(board);
                            }

                            let measured_latency = if latency.is_empty() {
                                0
                            } else {
                                latency.parse().expect("Error parsing server-reported latency")
                            };

                            (
                                search::best_move(ctx, start_time, measured_latency, slot).best_move,
                                200,
                            )
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
