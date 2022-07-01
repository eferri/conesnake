use crate::api::GameApi;
use crate::board::Board;
use crate::util::{Coord, Error};

use serde::{Deserialize, Serialize};

use std::cmp::max;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rules {
    Solo,
    Standard,
    Wrapped,
    Royale,
    Constrictor,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Map {
    Standard,
    Empty,
    ArcadeMaze,
    Royale,
    HzInnerWall,
    HzRings,
    HzColumns,
    HzRiversBridges,
    HzSpiral,
    HzScatter,
    HzGrowBox,
    HzExpandBox,
    HzExpandScatter,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub api: GameApi,
    pub is_solo: bool,
    pub prev_boards: Vec<Board>,
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
        let rules = req_game.ruleset.name;
        let map = req_game.map;

        match rules {
            Rules::Solo | Rules::Standard | Rules::Wrapped => (),
            _ => return Err(Error::BadBoardReq(format!("unsupported game mode {:?}", rules))),
        };

        match map {
            Map::Standard | Map::Empty | Map::ArcadeMaze => (),
            _ => return Err(Error::BadBoardReq(format!("unsupported game map {:?}", map))),
        };

        Ok(Game {
            api: req_game,
            is_solo: solo,
            prev_boards: Vec::new(),
        })
    }

    pub fn reset(&mut self) {
        self.prev_boards.clear();
    }

    pub fn add_board(&mut self, board: Board) {
        self.prev_boards.push(board);
    }

    pub fn start_board(&self) -> &Board {
        self.prev_boards.last().unwrap()
    }

    pub fn max_turn(&self, board: &Board) -> i32 {
        if let Map::ArcadeMaze = self.api.map {
            (213 - 3) * 100
        } else {
            (board.len() - 3) * 100
        }
    }

    pub fn score(&self, board: &Board, snake_idx: usize, depth: i32) -> f64 {
        if self.is_solo {
            let start_board = &self.prev_boards.last().unwrap();
            let start_health = start_board.snakes[snake_idx].health;
            let start_len = start_board.snakes[snake_idx].len;
            let len = board.snakes[snake_idx].len;

            let best_len = start_len + (depth + (100 - start_health)) / 100;
            1.0 - 0.1 * max((len - best_len).abs(), 10) as f64
        } else if board.snakes[snake_idx].alive {
            1.0
        } else {
            0.0
        }
    }

    pub fn over(&self, board: &Board) -> bool {
        !board.snakes[0].alive || board.num_alive_snakes() <= self.search_cutoff()
    }

    pub fn search_cutoff(&self) -> i32 {
        match (self.api.ruleset.name, self.is_solo) {
            (_, true) => 0,
            (Rules::Solo, _) => 0,
            (Rules::Standard, false) => 1,
            _ => 1,
        }
    }
}
