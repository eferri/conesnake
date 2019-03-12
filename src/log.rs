use env_logger::Env;

fn log_env() -> Env<'static> {
    Env::default()
        .default_filter_or("info")
        .default_write_style_or("always")
}

pub fn log_init() {
    env_logger::init_from_env(log_env());
}

pub fn log_test_init() {
    let _res = env_logger::Builder::from_env(log_env()).is_test(true).try_init();
}
