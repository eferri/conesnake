use clap::{Parser, ValueEnum};
use num_cpus;

pub const DEFAULT_TEMP: f64 = 1.1;
pub const DEFAULT_LOSS: f64 = 0.2;

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Worker,
    Relay,
}

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about = "conesnake", long_about = None)]
pub struct Config {
    #[clap(value_enum, default_value = "worker")]
    pub mode: Mode,

    #[clap(long)]
    pub worker: Vec<String>,

    #[clap(long, default_value = "8080")]
    pub port: String,

    #[clap(long, default_value_t = 8)]
    pub num_relay_reqs: usize,

    #[clap(long, default_value_t = num_cpus::get())]
    pub num_worker_threads: usize,

    #[clap(long, default_value_t = 8)]
    pub num_server_threads: usize,

    #[clap(long, default_value_t = 10000)]
    pub max_boards: usize,

    #[clap(long, default_value_t = 11)]
    pub max_width: i32,

    #[clap(long, default_value_t = 11)]
    pub max_height: i32,

    #[clap(long, default_value_t = 5)]
    pub max_snakes: i32,

    // Latencies are round trip time
    #[clap(long, default_value_t = 10)]
    pub latency: i32,

    #[clap(long, default_value_t = 20)]
    pub worker_latency: i32,

    // Algorithm parameters
    #[clap(long, default_value_t = DEFAULT_TEMP)]
    pub temperature: f64,

    #[clap(long, default_value_t = DEFAULT_LOSS)]
    pub virtual_loss: f64,
}
