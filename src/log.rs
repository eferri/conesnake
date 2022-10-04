use env_logger::Env;

fn log_env() -> Env<'static> {
    Env::default()
        .default_filter_or("info")
        .default_write_style_or("always")
}

pub fn log_init() {
    let _res = env_logger::Builder::from_env(log_env())
        .format_timestamp_millis()
        .try_init();
}

pub fn log_test_init() {
    let _res = env_logger::Builder::from_env(log_env())
        .format_timestamp_millis()
        .is_test(true)
        .try_init();
}
