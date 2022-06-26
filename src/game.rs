use crate::api::GameApi;
use crate::board::Board;
use crate::util::{Coord, Error};

use serde::{Deserialize, Serialize};

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
    pub fallback_latency: i32,
    pub latency_safety: i32,
    pub prev_delay: f64,
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
    pub const ADAPTIVE_SAFETY_MS: i32 = 10;

    pub fn from_req(req_game: GameApi, fallback_latency: i32, latency_safety: i32, solo: bool) -> Result<Self, Error> {
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
            fallback_latency,
            latency_safety,
            prev_boards: Vec::new(),
            prev_delay: 0.0,
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
            let max_depth = 250.0;
            let depth = (depth as f64).min(max_depth);
            depth / (max_depth)
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

    pub fn next_delay_us(&mut self, measured_latency_ms: i32) -> i64 {
        let target_latency_ms = self.api.timeout - self.latency_safety;
        if measured_latency_ms == 0 {
            self.prev_delay = target_latency_ms as f64 - self.fallback_latency as f64;
        } else {
            let error_ms = target_latency_ms as f64 - measured_latency_ms as f64;
            self.prev_delay += error_ms;
        }
        (self.prev_delay * 1000.0).round() as i64
    }
}
