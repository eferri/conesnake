use conesnake::api::MoveResp;
use conesnake::board::Board;
use conesnake::config::Mode;
use conesnake::game::{Map, Rules};
use conesnake::log::log_test_init;
use conesnake::server::Server;
use conesnake::tests::common::{get_config, test_game};
use conesnake::util::Move;

use futures::future::join_all;
use log::info;
use nix::sys::signal;
use pretty_assertions::{assert_eq, assert_ne};
use tokio::{task, time};

use std::time::Duration;

type TestCase<'a> = &'a [(bool, Move)];

const TESTS: &[(&str, TestCase, Rules, Map)] = &[(
    // This board was causing the search to over-prioritize the tie that is possible
    // When 0 moves down here. In practice this is extremely unlikely, so should be ignored
    "
    turn: 107 health: 66 health: 97 health: 85
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - - - - - - - - - -
    - - - - - - 1 < - - -
    - - - v < - - ^ < - -
    - - - v ^ d - > ^ > 0
    - - - v ^ < - ^ - ^ +
    - - - > v - - ^ - ^ <
    - - - - > 2 - ^ - - ^
    - - - - - b > ^ - b ^
    - - - - - - + - - - -
",
    &[(true, Move::Up)],
    Rules::Standard,
    Map::Standard,
)];

#[tokio::test(flavor = "multi_thread")]
async fn server_test() {
    log_test_init();

    let base_port = 4000;

    let base_config = get_config();

    let mut relay_config = base_config.clone();

    let num_workers = 3;
    let mut worker_addrs = Vec::new();
    let mut join_handles = Vec::new();

    for i in 0..num_workers {
        let mut worker_config = base_config.clone();
        worker_config.mode = Mode::Worker;

        let port = base_port + i + 1;

        worker_config.port = port.to_string();
        worker_addrs.push(format!("http://localhost:{port}").to_owned());

        join_handles.push(task::spawn(async move {
            let worker = Server::new(worker_config);
            worker.run().await
        }));
    }

    relay_config.worker_node = vec!["localhost".to_owned()];
    relay_config.worker_pod = worker_addrs;
    relay_config.port = base_port.to_string();
    relay_config.mode = Mode::Relay;
    relay_config.num_parallel_reqs = 1;

    join_handles.push(task::spawn(async move {
        let relay = Server::new(relay_config);
        relay.run().await
    }));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(2000))
        .build()
        .unwrap();

    // Wait for server to be ready
    loop {
        let res = client.get(format!("http://localhost:{base_port}/ping")).send().await;

        if res.is_err() || res.as_ref().unwrap().status() != 200 {
            time::sleep(Duration::from_millis(500)).await;
            continue;
        } else {
            break;
        }
    }

    time::sleep(Duration::from_millis(1000)).await;

    info!("All server tasks ready");

    for (board_str, desired_moves, rules, map) in TESTS.iter() {
        let mut game = test_game();
        game.ruleset = *rules;
        game.api.map = *map;

        let mut board = Board::from_str(board_str, &game).unwrap().to_req(&game).unwrap();

        board.you.latency = "0".to_owned();
        board.board.snakes[0].latency = "0".to_owned();

        let mut move_board = board.clone();
        move_board.turn = 1;

        let mut end_board = board.clone();
        end_board.turn = 2;

        info!("server_test: /start");

        client
            .post(format!("http://localhost:{base_port}/start"))
            .json(&board)
            .send()
            .await
            .unwrap();

        time::sleep(Duration::from_millis(500)).await;
        info!("server_test: /move");

        let move_response = client
            .post(format!("http://localhost:{base_port}/move"))
            .json(&move_board)
            .send()
            .await
            .unwrap()
            .json::<MoveResp>()
            .await
            .unwrap();

        for (eq, mv) in *desired_moves {
            if *eq {
                assert_eq!(*mv, move_response.mv);
            } else {
                assert_ne!(*mv, move_response.mv);
            }
        }

        time::sleep(Duration::from_millis(500)).await;

        info!("server_test: /end");

        client
            .post(format!("http://localhost:{base_port}/end"))
            .json(&end_board)
            .send()
            .await
            .unwrap();
    }

    signal::raise(signal::SIGTERM).unwrap();
    join_all(join_handles).await;
}
