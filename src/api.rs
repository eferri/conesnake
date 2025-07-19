use crate::config::MAX_SNAKES;
use crate::game::Map;
use crate::util::{Coord, Move};

use serde::{Deserialize, Serialize};

use std::fmt;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoyaleSettings {
    pub shrink_every_n_turns: i32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SquadSettings {
    pub allow_body_collisions: bool,
    pub shared_elimination: bool,
    pub shared_health: bool,
    pub shared_length: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub food_spawn_chance: i32,
    pub minimum_food: i32,
    pub hazard_damage_per_turn: i32,
    pub royale: RoyaleSettings,
    pub squad: SquadSettings,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Ruleset {
    pub name: String,
    pub version: String,
    pub settings: Settings,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GameApi {
    pub id: String,
    pub timeout: i32,
    pub source: String,
    pub map: Map,
    pub ruleset: Ruleset,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct ApiCoord {
    pub x: i8,
    pub y: i8,
}

impl ApiCoord {
    pub fn new(x: i8, y: i8) -> Self {
        ApiCoord { x, y }
    }

    pub fn to_internal(&self) -> Coord {
        Coord::new(self.x, self.y)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnakeApi {
    pub id: String,
    pub name: String,
    pub health: i32,
    pub body: Vec<ApiCoord>,
    pub latency: String,
    pub head: ApiCoord,
    pub length: i32,
    pub shout: Option<String>,
    pub squad: String,
    pub customizations: Customizations,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Customizations {
    pub color: Option<String>,
    pub head: Option<String>,
    pub tail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardApi {
    pub height: i32,
    pub width: i32,
    pub food: Vec<ApiCoord>,
    pub hazards: Vec<ApiCoord>,
    pub snakes: Vec<SnakeApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleState {
    pub game: GameApi,
    pub turn: i32,
    pub board: BoardApi,
    pub you: SnakeApi,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexResp {
    pub apiversion: String,
    pub author: String,
    pub color: String,
    pub head: String,
    pub tail: String,
    pub version: String,
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct SearchScore {
    pub score: f64,
    pub games: i64,
}
pub type Scores = [SearchScore; 4];

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchStats {
    pub total_nodes: i64,
    pub num_searches: i64,
    pub num_terminal: i64,
    pub total_playouts: i64,
    pub total_turns: i64,
    pub avg_playout_ns: f64,
    pub avg_turn_ns: f64,
    pub max_depth: i32,
    pub num_snakes: i32,
    pub scores: [Scores; MAX_SNAKES],
}

impl fmt::Display for SearchStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        writeln!(f, "total_nodes: {}", self.total_nodes)?;
        writeln!(f, "num_searches: {}", self.num_searches)?;
        writeln!(f, "num_terminal: {}", self.num_terminal)?;
        writeln!(f, "total_playouts: {}", self.total_playouts)?;
        writeln!(f, "total_turns: {}", self.total_turns)?;
        writeln!(f, "avg_playout_ns: {:.2}", self.avg_playout_ns)?;
        writeln!(f, "avg_turn_ns: {:.2}", self.avg_turn_ns)?;
        writeln!(f, "max_depth: {}", self.max_depth)?;
        writeln!(f, "num_snakes: {}", self.num_snakes)?;

        for i in 0..self.num_snakes {
            writeln!(f, "snake {i}:")?;
            for j in 0..4 {
                let mv_str = match j {
                    0 => "left",
                    1 => "right",
                    2 => "up",
                    _ => "down",
                };
                let score = self.scores[i as usize][j].score;
                let games = self.scores[i as usize][j].games;
                let avg = if games > 0 { score / games as f64 } else { 0.0 };

                writeln!(
                    f,
                    "    {mv_str} score: {score:.1} games: {games} avg: {avg:.5}",
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveResp {
    #[serde(rename = "move")]
    pub mv: Move,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scores: Option<Scores>,
}
