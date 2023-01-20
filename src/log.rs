use tracing_subscriber::{filter::LevelFilter, EnvFilter};

pub fn log_init() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .compact()
        .init()
}

pub fn log_test_init() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .compact()
        .try_init()
        .unwrap_or_default();
}
