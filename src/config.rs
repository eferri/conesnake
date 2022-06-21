use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about = "treesnake", long_about = None)]
pub struct Config {
    #[clap(long, default_value = "8080")]
    pub port: String,

    #[clap(long, default_value_t = 4)]
    pub num_threads: usize,

    #[clap(long, default_value_t = 1)]
    pub num_requests: usize,

    #[clap(long, default_value_t = 10000)]
    pub max_boards: usize,

    #[clap(long, default_value_t = 11)]
    pub max_width: i32,

    #[clap(long, default_value_t = 11)]
    pub max_height: i32,

    #[clap(long, default_value_t = 5)]
    pub max_snakes: i32,

    #[clap(long, default_value_t = 1.1)]
    pub temperature: f64,

    #[clap(long, default_value_t = 10)]
    pub fallback_latency: i32,

    #[clap(long, default_value_t = 5)]
    pub latency_safety: i32,

    #[clap(long)]
    pub certificate: Option<String>,

    #[clap(long)]
    pub private_key: Option<String>,

    #[clap(long)]
    pub always_sleep: bool,
}
