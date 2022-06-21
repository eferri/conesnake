use treesnake::api;

use treesnake::config::Config;
use treesnake::server::Server;
use treesnake::util::Move;

use env_logger::Env;

use log::info;

use std::fs;
use std::sync;
use std::thread;
use std::time;

fn init_test_logging() {
    let env = Env::default()
        .default_filter_or("info")
        .default_write_style_or("always");

    env_logger::init_from_env(env);
}

#[test]
fn index_test() {
    init_test_logging();

    let server = sync::Arc::new(Server::new(Config {
        port: "4000".to_owned(),
        certificate: None,
        private_key: None,
        num_threads: 1,
        num_requests: 1,
        max_boards: 100000,
        max_width: 4,
        max_height: 4,
        max_snakes: 1,
        temperature: 0.86,
        fallback_latency: 10,
        latency_safety: 5,
        always_sleep: true,
    }));
    server.start_server();

    let mut start_board: api::BattleState =
        serde_json::from_str(&fs::read_to_string("tests/data/start_basic_game.json").unwrap()).unwrap();

    start_board.you.latency = "0".to_owned();
    start_board.board.snakes[0].latency = "0".to_owned();

    let mut move_1_board = start_board.clone();
    move_1_board.turn = 1;

    let mut end_board = start_board.clone();
    end_board.turn = 2;

    let max_wait = time::Duration::from_secs(1);
    let mut total_dur = time::Duration::from_secs(0);
    while !server.is_ready() && total_dur < max_wait {
        let wait_dur = time::Duration::from_millis(100);
        thread::sleep(wait_dur);
        total_dur += wait_dur;
    }

    info!("server_test: /start");

    ureq::post("http://127.0.0.1:4000/start")
        .timeout(time::Duration::from_millis(500))
        .send_string(&serde_json::to_string(&start_board).unwrap())
        .unwrap();

    info!("server_test: /move");

    let response = ureq::post("http://127.0.0.1:4000/move")
        .timeout(time::Duration::from_millis(500))
        .send_string(&serde_json::to_string(&move_1_board).unwrap())
        .unwrap();

    let move_response: api::MoveResp = serde_json::from_str(&response.into_string().unwrap()).unwrap();

    assert!(move_response.mv == Move::Left || move_response.mv == Move::Up);

    info!("server_test: /end");

    ureq::post("http://127.0.0.1:4000/end")
        .timeout(time::Duration::from_millis(500))
        .send_string(&serde_json::to_string(&end_board).unwrap())
        .unwrap();
}
