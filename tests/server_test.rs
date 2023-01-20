use conesnake::api::BattleState;
use conesnake::api::MoveResp;
use conesnake::log::log_test_init;
use conesnake::server::Server;
use conesnake::tests::common::get_config;
use conesnake::util::Move;

use log::info;
use tokio::task;

use std::fs;
use std::time::Duration;

#[tokio::test]
async fn index_test() {
    log_test_init();

    let mut config = get_config();
    config.port = "4000".to_owned();
    let server = Server::new(config);

    task::spawn(async move { server.run().await });

    let mut start_board: BattleState =
        serde_json::from_str(&fs::read_to_string("tests/data/start_basic_game.json").unwrap()).unwrap();

    start_board.you.latency = "0".to_owned();
    start_board.board.snakes[0].latency = "0".to_owned();

    let mut move_1_board = start_board.clone();
    move_1_board.turn = 1;

    let mut end_board = start_board.clone();
    end_board.turn = 2;

    info!("server_test: /start");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();

    client
        .post("http://localhost:4000/start")
        .json(&start_board)
        .send()
        .await
        .unwrap();

    info!("server_test: /move");

    let move_response = client
        .post("http://localhost:4000/move")
        .json(&move_1_board)
        .send()
        .await
        .unwrap()
        .json::<MoveResp>()
        .await
        .unwrap();

    assert!(move_response.mv == Move::Left || move_response.mv == Move::Up);

    info!("server_test: /end");

    client
        .post("http://localhost:4000/end")
        .json(&end_board)
        .send()
        .await
        .unwrap();
}
