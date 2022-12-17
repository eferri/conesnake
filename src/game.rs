use crate::api::GameApi;
use crate::board::Board;
use crate::util::{Coord, Error};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rules {
    #[default]
    Solo,
    Standard,
    Wrapped,
    Constrictor,
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Map {
    #[default]
    Standard,
    Empty,
    ArcadeMaze,
    Royale,
    HzRiversBridges,
    HzRiversBridgesLg,
    HzIslandsBridges,
    HzIslandsBridgesLg,
}

#[derive(Debug, Default, Clone)]
pub struct Game {
    pub api: GameApi,
    pub ruleset: Rules,
    pub is_solo: bool,
}

pub const ARCADE_FOOD_COORDS: [Coord; 12] = [
    Coord { x: 1, y: 1 },
    Coord { x: 3, y: 11 },
    Coord { x: 4, y: 7 },
    Coord { x: 4, y: 17 },
    Coord { x: 9, y: 1 },
    Coord { x: 9, y: 5 },
    Coord { x: 9, y: 11 },
    Coord { x: 9, y: 17 },
    Coord { x: 14, y: 7 },
    Coord { x: 14, y: 17 },
    Coord { x: 15, y: 11 },
    Coord { x: 17, y: 1 },
];

impl Game {
    pub fn new(req_game: GameApi, solo: bool) -> Result<Self, Error> {
        // Simulated move doesn't return ruleset name
        if req_game.ruleset.name.is_empty() {
            return Ok(Game {
                api: req_game,
                ruleset: Rules::Standard,
                is_solo: solo,
            });
        }

        let mut ruleset_str = "\"".to_owned();
        ruleset_str.push_str(&req_game.ruleset.name);
        ruleset_str.push('"');

        let ruleset = serde_json::from_str(&ruleset_str)?;
        Ok(Game {
            api: req_game,
            ruleset,
            is_solo: solo,
        })
    }

    pub fn max_turn(&self, board: &Board) -> i32 {
        (board.len() - 3) * 100
    }

    pub fn score(&self, board: &Board, snake_idx: usize) -> f64 {
        if self.is_solo {
            board.turn as f64 / self.max_turn(board) as f64
        } else if board.snakes[snake_idx].alive() {
            1.0
        } else {
            0.0
        }
    }

    pub fn over(&self, board: &Board) -> bool {
        !board.snakes[0].alive() || board.num_alive_snakes() <= self.search_cutoff()
    }

    pub fn search_cutoff(&self) -> i32 {
        match (self.ruleset, self.is_solo) {
            (_, true) => 0,
            (Rules::Solo, _) => 0,
            (Rules::Standard, false) => 1,
            _ => 1,
        }
    }
}
