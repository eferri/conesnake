use crate::api::GameApi;
use crate::board::{Board, HeadOnCol};
use crate::config::Config;
use crate::util::{Coord, Error, Move};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rules {
    #[default]
    Solo,
    Standard,
    Wrapped,
    Constrictor,
    Royale,
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

    // Not currently used
    pub fn approx_score(&self, board: &Board, _cfg: &Config, snake_idx: usize, root_num_alive: i32) -> f64 {
        // TODO: replace if using
        let base_reward = 0.0;
        let len_reward = 0.0;
        let elim_reward = 0.0;
        let head_coll_reward = 0.0;
        let head_elim_reward = 0.0;

        if !board.snakes[snake_idx].alive() {
            if self.is_solo {
                return board.turn as f64 / self.max_turn(board) as f64;
            } else {
                return 0.0;
            }
        }

        let mut score = base_reward;

        let our_len = board.snakes[snake_idx].body.len();
        let num_alive = board.num_alive_snakes();

        for s_idx in 0..board.num_snakes() as usize {
            if s_idx == snake_idx {
                continue;
            }
            if our_len > board.snakes[s_idx].body.len() {
                score += len_reward;
            }

            score += (root_num_alive - num_alive) as f64 * elim_reward;
        }

        for mv_idx in 0..4 {
            if !board.valid_move(self, snake_idx, Move::from_idx(mv_idx)) {
                continue;
            }

            match board.head_on_col(self, snake_idx, Move::from_idx(mv_idx)) {
                HeadOnCol::PossibleCollision => score += head_coll_reward,
                HeadOnCol::PossibleElimination => score += head_elim_reward,
                HeadOnCol::None => (),
            }
        }

        score.clamp(0.0, 1.0)
    }

    pub fn over(&self, board: &Board) -> bool {
        board.num_alive_snakes() < self.min_alive_snakes()
    }

    pub fn min_alive_snakes(&self) -> i32 {
        match (self.ruleset, self.is_solo) {
            (_, true) => 1,
            (Rules::Solo, _) => 1,
            (_, false) => 2,
        }
    }
}
