use crate::api::BattleState;
use crate::board::Board;
use crate::game::Game;
use crate::util::Move;

use std::{
    io::Write,
    process::{Child, ChildStdout, Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_json::{de::IoRead, Deserializer, StreamDeserializer};

#[derive(Serialize, Deserialize)]
struct MoveState {
    pub request: BattleState,
    pub moves: Vec<Move>,
}

pub struct RefMove<'de> {
    child: Child,
    ref_iter: StreamDeserializer<'de, IoRead<ChildStdout>, BattleState>,
}

impl<'de> RefMove<'de> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut child = Command::new("move")
            .arg("move")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Error spawning rules process");
        let stdout = child.stdout.take().unwrap();
        let deserializer = Deserializer::from_reader(stdout);
        let ref_iter = deserializer.into_iter::<BattleState>();

        RefMove { child, ref_iter }
    }

    pub fn gen_ref_board(&mut self, game: &Game, board: &Board, moves: &[Move]) -> Board {
        let req = board.to_req(game).unwrap();
        let state = MoveState {
            request: req,
            moves: moves.to_vec(),
        };

        let state_bytes = serde_json::to_string(&state).unwrap().into_bytes();
        let written_bytes = self.child.stdin.as_mut().unwrap().write(&state_bytes).unwrap();
        assert_eq!(state_bytes.len(), written_bytes);

        let ref_state_res = self.ref_iter.next().unwrap();
        let mut ref_board = Board::from_req(
            game,
            ref_state_res.as_ref().unwrap(),
            board.width,
            board.height,
            board.max_snakes(),
        )
        .unwrap();

        // Patch "eliminated" which is not in the API
        for snake_idx in 0..ref_board.num_snakes() as usize {
            let snake = &mut ref_board.snakes[snake_idx];

            if snake.health == 0 {
                snake.eliminated = true
            }
        }

        ref_board
    }
}

impl<'de> Drop for RefMove<'de> {
    fn drop(&mut self) {
        self.child.wait().unwrap();
    }
}
