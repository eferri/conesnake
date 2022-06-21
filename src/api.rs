use crate::game::{Map, Rules};
use crate::util::{Coord, Move};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoyaleSettings {
    pub shrink_every_n_turns: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SquadSettings {
    pub allow_body_collisions: bool,
    pub shared_elimination: bool,
    pub shared_health: bool,
    pub shared_length: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub food_spawn_chance: i32,
    pub minimum_food: i32,
    pub hazard_damage_per_turn: i32,
    pub royale: RoyaleSettings,
    pub squad: SquadSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruleset {
    pub name: Rules,
    pub version: String,
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameApi {
    pub id: String,
    pub timeout: i32,
    pub source: String,
    pub map: Map,
    pub ruleset: Ruleset,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnakeApi {
    pub id: String,
    pub name: String,
    pub health: i32,
    pub body: Vec<Coord>,
    pub latency: String,
    pub head: Coord,
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
    pub food: Vec<Coord>,
    pub hazards: Vec<Coord>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveResp {
    #[serde(rename = "move")]
    pub mv: Move,
}
