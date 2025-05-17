use crate::api::{BattleState, IndexResp, MoveResp, Scores};
use crate::board::Board;
use crate::config::{Config, Mode, MAX_HEIGHT, MAX_SNAKES, MAX_WIDTH};
use crate::game::Game;
use crate::pool::ThreadPool;
use crate::rand::FastRand;
use crate::search;
use crate::search::{max_node_children, Node, SearchContext};
use crate::util::{Error, Move};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    serve, Json, Router,
};
use futures::future;
use log::{debug, error, info, warn};
use reqwest::Client;
use tokio::{net::TcpSocket, select, signal, time};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use std::{
    collections::HashMap,
    env, mem,
    net::SocketAddr,
    sync::atomic::{AtomicI64, Ordering},
    sync::Arc,
    time::{Duration, Instant},
};

pub struct Server {
    state: Arc<ServerState>,
}

struct ServerState {
    config: Config,
    req_client: Client,
    worker_map: HashMap<String, Vec<String>>,
    index: usize,

    worker_pool: ThreadPool,
    context: Arc<SearchContext<FastRand>>,

    max_nodes: AtomicI64,
}

impl Server {
    pub fn new(config: Config) -> Self {
        let mut worker_map = HashMap::new();

        // Associate each worker pod with a node
        for n in &config.worker_node {
            worker_map.insert(n.clone(), Vec::with_capacity(config.worker_pod.len()));
        }

        for w in &config.worker_pod {
            for n in &config.worker_node {
                if w.contains(n) {
                    let node_workers = worker_map.get_mut(n).unwrap();
                    node_workers.push(w.clone());
                    continue;
                }
            }
        }

        // Ensure all relay replicas have the same pod order
        for v in worker_map.values_mut() {
            v.sort();
        }

        if config.mode != Mode::Relay && config.max_boards > 0 {
            let node_size = mem::size_of::<Node<{ max_node_children(MAX_SNAKES as i32) }>>();
            let num_boards = config.max_boards;
            let space_size = node_size as i64 * num_boards as i64;

            info!("Size of Node: {}B", node_size);

            info!("Approx. size of search space: {}MiB", space_size >> 20);
            info!("Approx. size of search space: {}GiB", space_size >> 30);
        }

        info!(
            "Starting search space allocation max_boards: {}, width: {}, height: {}, max_snakes {}",
            config.max_boards, MAX_WIDTH, MAX_HEIGHT, MAX_SNAKES
        );

        let pod_name = match env::var("POD_NAME") {
            Ok(str) => str,
            Err(e) => {
                info!("Error retrieving POD_NAME: {}", e);
                "test-0".to_owned()
            }
        };

        let index = match pod_name.split('-').next_back().unwrap().parse() {
            Ok(idx) => idx,
            Err(e) => {
                error!("Error parsing pod index: {}", e);
                0
            }
        };

        Server {
            state: Arc::new(ServerState {
                config: config.clone(),
                worker_map,
                index,
                req_client: Client::new(),
                worker_pool: ThreadPool::new(config.num_threads),
                max_nodes: AtomicI64::new(0),
                context: Arc::new(SearchContext::new(&config)),
            }),
        }
    }

    pub async fn run(&self) {
        info!("Allocation complete");

        let app = Router::new()
            .route("/", get(root))
            .route("/ping", get(ping))
            .route("/trace", get(trace))
            .route("/move", post(move_req))
            .route("/start", post(start))
            .route("/end", post(end))
            .with_state(self.state.clone())
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            );

        let addr = SocketAddr::from(([0, 0, 0, 0], self.state.config.port.parse().unwrap()));

        let socket = TcpSocket::new_v4().unwrap();
        socket.set_reuseaddr(true).unwrap();
        socket.set_nodelay(true).unwrap();
        socket.bind(addr).unwrap();

        let listener = socket.listen(1024).unwrap();

        info!("Starting conesnake");

        serve(listener, app)
            .with_graceful_shutdown(async {
                let ctrl_c = signal::ctrl_c();

                let mut term_stream = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
                let terminate = term_stream.recv();

                select! {
                    _ = ctrl_c => {},
                    _ = terminate => {},
                }
            })
            .await
            .unwrap();

        info!("Exiting conesnake");
    }
}

async fn root() -> Response {
    Json(IndexResp {
        apiversion: "1".to_owned(),
        author: "conesnake".to_owned(),
        color: "#C42B3A".to_owned(),
        head: "sand-worm".to_owned(),
        tail: "fat-rattle".to_owned(),
        version: ("1.0.0").to_owned(),
    })
    .into_response()
}

async fn start(State(state): State<Arc<ServerState>>, Json(game_state): Json<BattleState>) -> Response {
    if let Mode::Relay = state.config.mode {
        info!("game ID: {}", game_state.game.id);
        info!("rules: {:?}", game_state.game.ruleset.name);
        info!("map: {:?}", game_state.game.map);
        info!("timeout: {}", game_state.game.timeout);
        info!(
            "width: {}, height: {}, snakes: {}",
            game_state.board.width,
            game_state.board.height,
            game_state.board.snakes.len()
        );
        info!(
            "food spawn chance: {}",
            game_state.game.ruleset.settings.food_spawn_chance
        );
        info!("min food: {}", game_state.game.ruleset.settings.minimum_food);
    }

    StatusCode::OK.into_response()
}

async fn end() -> Response {
    StatusCode::OK.into_response()
}

async fn ping(State(state): State<Arc<ServerState>>) -> Response {
    if let Mode::Worker = state.config.mode {
        return StatusCode::OK.into_response();
    }

    let mut ping_futures = Vec::with_capacity(state.config.worker_pod.len());

    for worker in &state.config.worker_pod {
        ping_futures.push(async {
            let worker = worker.clone();

            let req_start = Instant::now();
            let res = state
                .req_client
                .get(format!("{worker}/"))
                .timeout(Duration::from_millis(600))
                .send()
                .await;

            let req_dur = Instant::now() - req_start;
            let latency_ms = req_dur.as_millis() as i32;

            match res {
                Err(e) => {
                    warn!("Error pinging worker: {}", e);
                    false
                }
                Ok(_) => {
                    info!("Worker {} ping latency ms: {}", worker, latency_ms);
                    true
                }
            }
        })
    }

    let timeout_dur = Duration::from_millis(700);

    let ping_res = match time::timeout(timeout_dur, future::join_all(ping_futures)).await {
        Ok(res) => res,
        Err(_) => {
            error!("Ping request hang");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if ping_res.iter().any(|x| *x) {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
    .into_response()
}

async fn trace() -> Response {
    info!("Tracing request");
    StatusCode::OK.into_response()
}

async fn move_req(State(state): State<Arc<ServerState>>, Json(game_state): Json<BattleState>) -> Response {
    let start_time = Instant::now();

    let api_latency = game_state.you.latency.clone();
    let timeout = game_state.game.timeout;

    let game_width = game_state.board.width;
    let game_height = game_state.board.height;
    let game_snakes = game_state.board.snakes.len() as i32;

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
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    if (MAX_WIDTH as i32) < game_width || (MAX_HEIGHT as i32) < game_height || (MAX_SNAKES as i32) < game_snakes {
        error!(
            "Server not configured for w: {} h: {} max_snakes: {}",
            game_width, game_height, game_snakes
        );
        error!(
            "Current settings are max_width: {} max_height: {} max_snakes: {}",
            MAX_WIDTH, MAX_HEIGHT, MAX_SNAKES
        );
        return StatusCode::CONFLICT.into_response();
    }

    if let Mode::Relay = state.config.mode {
        info!("turn: {}", game_state.turn);
    }

    let board = match Board::from_req(&game, &game_state) {
        Err(e) => {
            error!("Error parsing board - {}", e);
            return StatusCode::BAD_REQUEST.into_response();
        }
        Ok(board) => board,
    };

    let resp = match state.config.mode {
        Mode::Worker => {
            let search_res = search::mcts(state.context.clone(), &state.worker_pool, &board, &game, start_time);

            match search_res {
                Ok(stats) => {
                    state.max_nodes.fetch_max(stats.total_nodes, Ordering::AcqRel);
                    let max = state.max_nodes.load(Ordering::Acquire);
                    info!("max nodes expanded: {}", max);
                    let mv = search::best_move(&state.config, 0, &stats.scores, false);
                    (
                        StatusCode::OK,
                        Json(MoveResp {
                            mv,
                            scores: Some(stats.scores[0]),
                        }),
                    )
                }
                Err(e) => {
                    error!("Error from mcts: {}", e);
                    (
                        StatusCode::CONFLICT,
                        Json(MoveResp {
                            mv: Move::Left,
                            scores: None,
                        }),
                    )
                }
            }
        }

        Mode::Relay => {
            let worker_res = run_workers(state.clone(), &game_state, start_time).await;
            match worker_res {
                Ok(scores) => {
                    let mv = search::best_move(&state.config, 0, &[scores], true);
                    info!("board:\n{}", board);
                    (StatusCode::OK, Json(MoveResp { mv, scores: None }))
                }
                Err(e) => {
                    error!("Error from run_workers: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(MoveResp {
                            mv: Move::Left,
                            scores: None,
                        }),
                    )
                }
            }
        }
    };

    resp.into_response()
}

async fn run_workers(state: Arc<ServerState>, game_state: &BattleState, start_time: Instant) -> Result<Scores, Error> {
    let delay_ms = game_state.game.timeout - state.config.latency;

    let mut worker_futures = Vec::with_capacity(state.config.worker_pod.len() / state.config.num_parallel_reqs);

    for node_pods in state.worker_map.values() {
        let num_pod_reqs = node_pods.len() / state.config.num_parallel_reqs;
        let pod_range = (state.index * num_pod_reqs)..((state.index + 1) * num_pod_reqs);

        for worker in &node_pods[pod_range] {
            worker_futures.push(async {
                let worker = worker.clone();
                let mut game_state = game_state.clone();

                game_state.you.latency = "0".to_owned();
                game_state.game.timeout = delay_ms;

                let current_dur = Instant::now() - start_time;
                let timeout_dur = Duration::from_millis(delay_ms as u64).saturating_sub(current_dur);

                debug!("Worker request timeout is {} us", timeout_dur.as_micros());

                let req_start = Instant::now();
                let move_resp = state
                    .req_client
                    .post(format!("{worker}/move"))
                    .timeout(timeout_dur)
                    .json(&game_state)
                    .send()
                    .await?
                    .json::<MoveResp>()
                    .await?;

                let req_dur = Instant::now() - req_start;
                let server_latency = req_dur.as_millis() as i64;
                let mut run_str = format!("\nWorker {worker} move latency ms {server_latency}\n");

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
                info!("{}\n", run_str);

                Ok(scores)
            });
        }
    }

    let err_timeout_dur = Duration::from_millis(delay_ms as u64 + 5);

    let worker_res: Vec<Result<Scores, Error>> =
        match time::timeout(err_timeout_dur, future::join_all(worker_futures)).await {
            Ok(res) => res,
            Err(_) => return Err(Error::RequestError("Worker move request hang".to_owned())),
        };

    let mut total_scores: Scores = Default::default();

    for res in worker_res.iter() {
        if let Err(e) = res {
            error!("Error from worker: {}", e);
        }
    }

    if worker_res.iter().all(|x| x.is_err()) {
        return Err(Error::RequestError("All worker requests failed".to_owned()));
    }

    for scores in worker_res.iter().flatten() {
        for (i, s) in total_scores.iter_mut().enumerate() {
            s.score += scores[i].score;
            s.games += scores[i].games;
        }
    }

    Ok(total_scores)
}
