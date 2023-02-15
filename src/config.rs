use crate::board::Board;
use crate::game::{Game, Rules};

use clap::{Parser, ValueEnum};

// Hyperparameters

pub const DEFAULT_TEMP: f64 = 2.0;

#[derive(ValueEnum, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Worker,
    Relay,
}

#[derive(Parser, Clone, Debug, Default)]
#[clap(author, version, about = "conesnake", long_about = None)]
pub struct Config {
    #[clap(value_enum, default_value = "worker")]
    pub mode: Mode,

    #[clap(long)]
    pub worker_node: Vec<String>,

    #[clap(long)]
    pub worker_pod: Vec<String>,

    #[clap(long, default_value = "8080")]
    pub port: String,

    #[clap(long, default_value_t = 1)]
    pub num_parallel_reqs: usize,

    #[clap(long, default_value_t = 8)]
    pub num_worker_threads: usize,

    #[clap(long, default_value_t = 10000)]
    pub max_boards: usize,

    #[clap(long, default_value_t = 11)]
    pub max_width: i32,

    #[clap(long, default_value_t = 11)]
    pub max_height: i32,

    #[clap(long, default_value_t = 5)]
    pub max_snakes: i32,

    // Latencies are round trip time
    #[clap(long, default_value_t = 30)]
    pub latency: i32,

    #[clap(long, default_value_t = false)]
    pub compare: bool,

    // Algorithm parameters
    #[clap(long, default_value_t = DEFAULT_TEMP)]
    pub temperature: f64,

    #[clap(long, default_value_t = 1.0)]
    pub multi_snake_addition: f64,

    #[clap(long, default_value_t = 3.0)]
    pub constrictor_mult: f64,

    #[clap(long, default_value_t = 0.1)]
    pub head_on_thresh: f64,

    #[clap(long, default_value_t = true)]
    pub strong_playout: bool,
}

impl Config {
    pub fn set_temp(&mut self, board: &Board, game: &Game) {
        match (board.num_alive_snakes(), game.ruleset) {
            (_, Rules::Solo) => self.temperature = self.temperature,
            (_, Rules::Constrictor) => self.temperature = self.temperature * self.constrictor_mult,
            (2, Rules::Standard) => self.temperature = self.temperature,
            (2, Rules::Royale) => self.temperature = self.temperature,
            (2, Rules::Wrapped) => self.temperature = self.temperature,
            (_, Rules::Standard) => self.temperature = self.temperature + self.multi_snake_addition,
            (_, Rules::Royale) => self.temperature = self.temperature + self.multi_snake_addition,
            (_, Rules::Wrapped) => self.temperature = self.temperature + self.multi_snake_addition,
        }
    }
}
