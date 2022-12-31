use crate::search;

use crate::api::{BattleState, IndexResp, MoveResp, Scores};
use crate::board::Board;
use crate::config::{Config, Mode};
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::rand::FastRand;
use crate::search::{Node, SearchContext};
use crate::util::{Error, Move};

use crossbeam_channel::{Receiver, Sender};
use deepsize::DeepSizeOf;
use log::{error, info, warn};
use tiny_http::{ConfigListenAddr, Method, Request, Response, StatusCode};

use std::{
    io::Cursor,
    sync::atomic::{AtomicBool, AtomicI64, Ordering},
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};

pub struct Server {
    server_pool: ThreadPool,
    worker_pool: Arc<ThreadPool>,
    state: Arc<ServerState>,
}

struct ServerState {
    config: Config,
    workers: Vec<String>,

    // search resources
    context: Arc<SearchContext<FastRand>>,

    // synchronization
    done_flag: AtomicBool,
    ready_flag: AtomicBool,
    server_barrier: Barrier,

    request_states: (Sender<ServerThreadState>, Receiver<ServerThreadState>),

    // game stats
    max_nodes: AtomicI64,
}

//

struct PingResults {
    healthy: bool,
}

struct WorkerResults {
    scores: Scores,
    healthy: bool,
}

struct ServerThreadState {
    ping_chans: (Sender<PingResults>, Receiver<PingResults>),
    worker_chans: (Sender<WorkerResults>, Receiver<WorkerResults>),
}

//

type WorkerPingResp = i32;

struct WorkerMoveResp {
    pub status: i32,
    pub scores: Scores,
}

//

type StrResponse = Response<Cursor<Vec<u8>>>;

impl Server {
    const TICK_MS: u64 = 100;

    pub fn new(config: Config) -> Self {
        let mut workers = Vec::new();

        for w in &config.worker {
            let strs: Vec<&str> = w.split(',').collect();
            let addr = strs[0];
            let worker_par_reqs = strs[1].parse::<i32>().unwrap();

            for _ in 0..worker_par_reqs {
                workers.push(addr.to_owned());
            }
        }

        if config.mode != Mode::Relay && config.max_boards > 0 {
            let test_node = Node::new(Board::new(0, 0, config.max_width, config.max_height, config.max_snakes));

            let node_size = test_node.deep_size_of();
            let num_boards = config.max_boards;
            let space_size = node_size as i64 * num_boards as i64;

            info!("Size of Node: {}B", node_size);

            info!("Approx. size of search space: {}MiB", space_size >> 20);
            info!("Approx. size of search space: {}GiB", space_size >> 30);
        }

        info!(
            "Starting search space allocation max_boards: {}, width: {}, height: {}, max_snakes {}",
            config.max_boards, config.max_width, config.max_height, config.max_snakes
        );

        let (request_sender, request_reciever) = crossbeam_channel::bounded(config.num_relay_reqs);
        for _ in 0..config.num_relay_reqs {
            request_sender
                .send(ServerThreadState {
                    ping_chans: crossbeam_channel::bounded(workers.len()),
                    worker_chans: crossbeam_channel::bounded(workers.len()),
                })
                .unwrap()
        }

        let num_threads = match config.mode {
            Mode::Relay => config.num_relay_reqs * workers.len(),
            Mode::Worker => config.num_worker_threads,
        };

        Server {
            server_pool: ThreadPool::new(config.num_server_threads),
            worker_pool: Arc::new(ThreadPool::new(num_threads)),

            state: Arc::new(ServerState {
                config: config.clone(),
                workers,

                done_flag: AtomicBool::new(false),
                ready_flag: AtomicBool::new(false),
                server_barrier: Barrier::new(config.num_server_threads + 1),
                request_states: (request_sender, request_reciever),

                max_nodes: AtomicI64::new(0),

                context: Arc::new(SearchContext::new(&config)),
            }),
        }
    }

    pub fn stop_server(&self) {
        info!("Stopping conesnake");
        self.state.done_flag.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.state.ready_flag.load(Ordering::Acquire)
    }

    pub fn wait_done(&self) {
        self.state.server_barrier.wait();
        info!("Exiting conesnake");
    }

    pub fn start_server(&self) {
        let addr = ConfigListenAddr::from_socket_addrs(format!("0.0.0.0:{}", self.state.config.port)).unwrap();
        let server = Arc::new(tiny_http::Server::new(tiny_http::ServerConfig { addr, ssl: None }).unwrap());

        info!("Started conesnake");
        self.state.ready_flag.store(true, Ordering::Release);

        for _ in 0..self.state.config.num_server_threads {
            let server = server.clone();
            let state = self.state.clone();
            let worker_pool = self.worker_pool.clone();

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
                        continue;
                    }

                    let start_time = Instant::now();

                    let mut request = request_opt.unwrap();
                    let method = request.method().clone();
                    let url = request.url().to_owned();

                    info!(" --- start {}, {}", method, url);

                    let addr = request.remote_addr().unwrap().to_string();
                    let headers = request.headers();

                    let mut forward_for = String::from("N/A");

                    for h in headers.iter() {
                        if h.field.as_str() == "X-Forwarded-For" {
                            forward_for = h.value.to_string();
                        }
                    }

                    let response = get_response(state.clone(), worker_pool.clone(), &mut request, start_time);
                    let code = response.status_code().0;

                    request.respond(response).unwrap();

                    let dur = (start_time.elapsed().as_micros() as f64) / 1000.0;
                    let msg = format!(
                        " --- end - {}, {}, code {}, addr {}, forward-for {}, duration {}ms\n\n",
                        method, url, code, addr, forward_for, dur
                    );

                    if code < 400 {
                        info!("{}", msg);
                    } else {
                        error!("{}", msg);
                    }
                }
                state.server_barrier.wait();
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
    pool: Arc<ThreadPool>,
    request: &mut Request,
    start_time: Instant,
) -> StrResponse {
    let (req_send, req_recv) = state.request_states.clone();
    let req_state = req_recv.recv().unwrap();

    let resp = match request.method() {
        Method::Get => match request.url() {
            "/" => StrResponse::from_string(
                serde_json::to_string(&IndexResp {
                    apiversion: "1".to_owned(),
                    author: "conesnake".to_owned(),
                    color: "#C42B3A".to_owned(),
                    head: "sand-worm".to_owned(),
                    tail: "fat-rattle".to_owned(),
                    version: ("1.0.0").to_owned(),
                })
                .unwrap(),
            )
            .with_status_code(200),
            "/ping" => {
                let code = if state.config.mode == Mode::Relay {
                    ping_workers(state, &req_state, &pool)
                } else {
                    200
                };

                StrResponse::from_string("").with_status_code(code)
            }
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

            let is_solo = game_state.board.snakes.len() == 1;
            let game_id = game_state.game.id.clone();

            let game = match Game::new(game_state.game.clone(), is_solo) {
                Ok(game) => game,
                Err(e) => {
                    error!("Error parsing game {} {}", game_id, e);
                    return StrResponse::from_string("{}").with_status_code(StatusCode(400));
                }
            };

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

                    info!("turn: {}", game_state.turn);

                    let parsed_board = Board::from_req(
                        &game,
                        &game_state,
                        state.config.max_width,
                        state.config.max_height,
                        state.config.max_snakes,
                    );

                    let (resp, code) = match parsed_board {
                        Err(e) => {
                            error!("Error parsing board - {}", e);
                            return StrResponse::from_string("{}").with_status_code(StatusCode(400));
                        }
                        Ok(board) => {
                            info!("board:\n{}", board);

                            let (resp, status) = match state.config.mode {
                                Mode::Worker => {
                                    let resp = worker_search(state, &pool, &board, &game, start_time);
                                    let mv = search::best_move(&resp.scores);

                                    (
                                        MoveResp {
                                            mv,
                                            scores: Some(resp.scores),
                                        },
                                        resp.status,
                                    )
                                }
                                Mode::Relay => {
                                    let resp = run_workers(state, &req_state, &pool, &game_state, start_time);
                                    let mv = search::best_move(&resp.scores);
                                    (MoveResp { mv, scores: None }, resp.status)
                                }
                            };

                            (resp, status)
                        }
                    };

                    let resp_str = serde_json::to_string(&resp).unwrap();
                    StrResponse::from_string(resp_str).with_status_code(code)
                }
                "/end" => StrResponse::from_string("{}").with_status_code(StatusCode(200)),
                _ => StrResponse::from_string("{}").with_status_code(StatusCode(404)),
            }
        }
        _ => StrResponse::from_string("{}").with_status_code(StatusCode(404)),
    };

    req_send
        .send(req_state)
        .unwrap_or_else(|err| error!("Error sending to req_state channel {}", err));

    resp
}

fn worker_search(
    state: Arc<ServerState>,
    pool: &ThreadPool,
    board: &Board,
    game: &Game,
    start_time: Instant,
) -> WorkerMoveResp {
    let ctx = state.context.clone();

    let search_res = search::search_moves(ctx, pool, board, game, start_time);

    let resp = match search_res {
        Ok(stats) => {
            state.max_nodes.fetch_max(stats.total_nodes, Ordering::AcqRel);

            WorkerMoveResp {
                scores: stats.scores,
                status: 200,
            }
        }
        Err(e) => {
            error!("Error getting search_moves: {}", e);
            WorkerMoveResp {
                scores: Default::default(),
                status: 500,
            }
        }
    };

    let max = state.max_nodes.load(Ordering::Acquire);
    info!("max nodes expanded: {}", max);

    resp
}

fn run_workers(
    state: Arc<ServerState>,
    req_state: &ServerThreadState,
    pool: &ThreadPool,
    game_state: &BattleState,
    start_time: Instant,
) -> WorkerMoveResp {
    let mut resp = WorkerMoveResp {
        scores: Default::default(),
        status: 500,
    };

    let delay_ms = game_state.game.timeout - state.config.latency;

    for i in 0..state.workers.len() {
        let state = state.clone();
        let mut game_state = game_state.clone();

        let worker_send = req_state.worker_chans.0.clone();

        pool.execute(move || {
            let worker = &state.workers[i];

            game_state.you.latency = "0".to_owned();
            game_state.game.timeout = delay_ms;

            let current_dur = Instant::now() - start_time;
            let timeout_dur = Duration::from_millis(delay_ms as u64).saturating_sub(current_dur);

            if timeout_dur.is_zero() {
                let run_scores = WorkerResults {
                    scores: Default::default(),
                    healthy: false,
                };
                worker_send.send(run_scores).unwrap();
                return;
            }

            let ureq_agent = ureq::builder()
                .timeout(timeout_dur)
                .timeout_connect(timeout_dur)
                .build();

            let req_start = Instant::now();
            let res = ureq_agent.post(&format!("{}/move", worker)).send_json(game_state);
            let req_dur = Instant::now() - req_start;
            let server_latency = req_dur.as_micros() as i64;

            let mut run_str = format!("\nWorker {} latency us {}\n", worker, server_latency);

            let run_scores = match res {
                Err(e) => {
                    error!("Error getting worker move: {}", e);
                    WorkerResults {
                        scores: Default::default(),
                        healthy: false,
                    }
                }
                Ok(resp) => {
                    let move_resp = resp.into_json::<MoveResp>().unwrap();
                    let scores = move_resp.scores.unwrap();
                    for (i, s) in scores.iter().enumerate() {
                        let score = if s.games == 0 { 0.0 } else { s.score / s.games as f64 };

                        run_str.push_str(&format!(
                            "move: {:<6} score: {:.8}  games: {}\n",
                            Move::from_idx(i),
                            score,
                            s.games
                        ));
                    }
                    WorkerResults { scores, healthy: true }
                }
            };

            info!("{}\n", run_str);

            worker_send.send(run_scores).unwrap();
        });
    }

    for _ in 0..(state.workers.len() as i32) {
        let recv_resp = req_state.worker_chans.1.recv();

        let worker_res = match recv_resp {
            Ok(s) => s,
            Err(e) => {
                error!("Worker channel error! {}", e);
                continue;
            }
        };

        if worker_res.healthy {
            resp.status = 200;
        }

        for (i, s) in resp.scores.iter_mut().enumerate() {
            s.score += worker_res.scores[i].score;
            s.games += worker_res.scores[i].games;
        }
    }

    resp
}

fn ping_workers(state: Arc<ServerState>, req_state: &ServerThreadState, pool: &ThreadPool) -> WorkerPingResp {
    for i in 0..state.workers.len() {
        let ping_send = req_state.ping_chans.0.clone();
        let state = state.clone();

        pool.execute(move || {
            let worker = &state.workers[i];

            let req_start = Instant::now();
            let res = ureq::get(&format!("{}/", worker))
                .timeout(Duration::from_millis(300))
                .call();
            let req_dur = Instant::now() - req_start;
            let latency_ms = req_dur.as_millis() as i32;

            let healthy = match res {
                Err(e) => {
                    error!("Error pinging worker: {}", e);
                    false
                }
                Ok(_) => {
                    info!("Worker {} latency ms: {}", worker, latency_ms);
                    true
                }
            };

            let resp = PingResults { healthy };

            ping_send.send(resp).unwrap();
        });
    }

    let mut resp_status = 500;

    for _ in 0..state.workers.len() {
        let worker_res = req_state.ping_chans.1.recv();
        let worker_resp = match worker_res {
            Ok(s) => s,
            Err(e) => {
                error!("Worker channel error: {}", e);
                continue;
            }
        };

        if worker_resp.healthy {
            resp_status = 200;
        }
    }

    resp_status
}
