use clap::{Parser, ValueEnum};

// Hyperparameters

pub const DEFAULT_TEMP: f64 = 4.0;
pub const DEFAULT_MIN_PLAYOUTS: i64 = 1;
pub const DEFAULT_WIN_VAL: f64 = 1.0;
pub const DEFAULT_LOSS_VAL: f64 = 0.0;
pub const DEFAULT_TIE_VAL: f64 = 0.0;

#[derive(ValueEnum, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Worker,
    Relay,
}

#[derive(Parser, Clone, Debug, Default)]
#[clap(author, version, about = "conesnake", long_about = None)]
pub struct Config {
    #[clap(long, value_enum, default_value = "worker")]
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
    pub num_threads: usize,

    #[clap(long, default_value_t = 10000)]
    pub max_boards: usize,

    #[clap(long, default_value_t = 11)]
    pub max_width: i32,

    #[clap(long, default_value_t = 11)]
    pub max_height: i32,

    #[clap(long, default_value_t = 5)]
    pub max_snakes: i32,

    // Latencies are round trip time
    #[clap(long, default_value_t = 50)]
    pub latency: i32,

    #[clap(long, default_value_t = false)]
    pub compare: bool,

    // Algorithm parameters
    #[clap(long, default_value_t = DEFAULT_TEMP)]
    pub temperature: f64,

    #[clap(long, default_value_t = DEFAULT_MIN_PLAYOUTS)]
    pub min_playouts: i64,

    #[clap(long, allow_negative_numbers = true, default_value_t = DEFAULT_WIN_VAL)]
    pub win_val: f64,

    #[clap(long, allow_negative_numbers = true, default_value_t = DEFAULT_LOSS_VAL)]
    pub loss_val: f64,

    #[clap(long, allow_negative_numbers = true, default_value_t = DEFAULT_TIE_VAL)]
    pub tie_val: f64,

    #[clap(long, default_value_t = false)]
    pub strong_playout: bool,
}
