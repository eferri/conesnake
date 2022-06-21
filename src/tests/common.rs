use crate::api::{GameApi, RoyaleSettings, Ruleset, Settings, SquadSettings};
use crate::config::Config;
use crate::game::{Game, Map, Rules};
use crate::search::SearchContext;

const TEST_TEMPERATURE: f64 = 1.1;

pub fn test_context() -> SearchContext {
    let context = SearchContext::new(Config {
        port: "".to_owned(),
        num_threads: 1,
        num_requests: 1,
        max_boards: 1000,
        max_width: 19,
        max_height: 21,
        max_snakes: 6,
        temperature: TEST_TEMPERATURE,
        fallback_latency: 10,
        latency_safety: 5,
        certificate: None,
        private_key: None,
        always_sleep: false,
    });

    context.allocate();
    context
}

pub fn release_context() -> SearchContext {
    let context = SearchContext::new(Config {
        port: "".to_owned(),
        num_threads: 24,
        num_requests: 1,
        max_boards: 375000,
        max_width: 19,
        max_height: 21,
        max_snakes: 5,
        temperature: TEST_TEMPERATURE,
        fallback_latency: 50,
        latency_safety: 100,
        certificate: None,
        private_key: None,
        always_sleep: false,
    });

    context.allocate();
    context
}

pub fn get_context() -> SearchContext {
    #[cfg(debug_assertions)]
    return test_context();
    #[cfg(not(debug_assertions))]
    return release_context();
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
        fallback_latency: 100,
        latency_safety: 5,
        prev_delay: 0.0,
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
