use crate::api::{GameApi, RoyaleSettings, Ruleset, Settings, SquadSettings};
use crate::config::{Config, Mode, DEFAULT_TEMP};
use crate::game::{Game, Map, Rules};
use crate::search::SearchContext;

pub fn test_config() -> Config {
    Config {
        mode: Mode::Local,
        worker: vec![],
        port: "".to_owned(),
        num_runs: 1,
        num_threads: 1,
        num_server_threads: 1,
        max_boards: 2000,
        max_width: 19,
        max_height: 21,
        max_snakes: 5,
        temperature: DEFAULT_TEMP,
        fallback_latency: 10,
        latency_safety: 5,
    }
}

pub fn release_config() -> Config {
    Config {
        mode: Mode::Local,
        worker: vec![],
        port: "".to_owned(),
        num_runs: 3,
        num_threads: 24,
        num_server_threads: 8,
        max_boards: 375000,
        max_width: 19,
        max_height: 21,
        max_snakes: 5,
        temperature: DEFAULT_TEMP,
        fallback_latency: 50,
        latency_safety: 100,
    }
}

pub fn test_context() -> SearchContext {
    let context = SearchContext::new(test_config());
    context.allocate();
    context
}

pub fn release_context() -> SearchContext {
    let context = SearchContext::new(test_config());
    context.allocate();
    context
}

pub fn get_context() -> SearchContext {
    #[cfg(debug_assertions)]
    return test_context();
    #[cfg(not(debug_assertions))]
    return release_context();
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
                name: Rules::Standard,
                version: "".to_owned(),
                settings: Settings {
                    food_spawn_chance: 0,
                    minimum_food: 0,
                    hazard_damage_per_turn: 100,
                    royale: RoyaleSettings {
                        shrink_every_n_turns: 0,
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
        is_solo: false,
        prev_boards: Vec::new(),
    }
}

pub fn solo_game() -> Game {
    let mut game = test_game();
    game.api.ruleset.name = Rules::Solo;
    game
}

pub fn wrapped_game() -> Game {
    let mut game = test_game();
    game.api.ruleset.name = Rules::Wrapped;
    game
}
