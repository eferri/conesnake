use crate::api::{Customizations, GameApi, RoyaleSettings, Ruleset, Settings, SnakeApi, SquadSettings};
use crate::config::Config;
use crate::game::{Game, Map, Rules};
use crate::rand::{FastRand, MaxRand};
use crate::search::SearchContext;
use crate::util::Coord;

use clap::Parser;
use log::info;

pub fn test_config() -> Config {
    info!("test_config");
    // Ensure clap defaults are used
    let mut config = Config::parse_from(["".to_owned()]);

    config.num_threads = 1;
    config.max_boards = 20000;
    config.latency = 50;

    config
}

pub fn release_config() -> Config {
    info!("release_config");
    let mut config = Config::parse_from(["".to_owned()]);

    config.num_threads = 8;
    config.max_boards = 350000;

    config
}

pub fn get_context() -> SearchContext<FastRand> {
    SearchContext::new(&get_config())
}

pub fn get_deterministic_context() -> SearchContext<MaxRand> {
    SearchContext::new(&get_config())
}

pub fn get_config() -> Config {
    #[cfg(debug_assertions)]
    return test_config();
    #[cfg(not(debug_assertions))]
    return release_config();
}

pub fn test_game() -> Game {
    Game {
        api: GameApi {
            id: "".to_owned(),
            timeout: 500,
            source: "".to_owned(),
            map: Map::Standard,
            ruleset: Ruleset {
                name: "standard".to_owned(),
                version: "".to_owned(),
                settings: Settings {
                    food_spawn_chance: 15,
                    minimum_food: 1,
                    hazard_damage_per_turn: 100,
                    royale: RoyaleSettings {
                        shrink_every_n_turns: 20,
                    },
                    squad: SquadSettings {
                        allow_body_collisions: false,
                        shared_elimination: false,
                        shared_health: false,
                        shared_length: false,
                    },
                },
            },
        },
        ruleset: Rules::Standard,
        is_solo: false,
    }
}

pub fn solo_game() -> Game {
    let mut game = test_game();
    game.ruleset = Rules::Solo;
    game.api.ruleset.name = "solo".to_owned();
    game
}

pub fn wrapped_game() -> Game {
    let mut game = test_game();
    game.ruleset = Rules::Wrapped;
    game.api.ruleset.name = "wrapped".to_owned();
    game
}

pub fn test_snake(coords: &[Coord], health: i32) -> SnakeApi {
    SnakeApi {
        id: "0".to_owned(),
        name: "conesnake".to_owned(),
        customizations: Customizations {
            color: None,
            head: None,
            tail: None,
        },
        body: coords.to_vec(),
        head: coords[0],
        health,
        latency: "10".to_owned(),
        length: coords.len() as i32,
        shout: None,
        squad: "".to_owned(),
    }
}
