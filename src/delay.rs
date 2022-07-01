#[derive(Debug, Clone)]
pub struct Delay {
    pub fallback_latency_ms: i32,
    pub latency_safety_ms: i32,
    pub prev_timeout: i32,
    pub prev_delay: f64,
}

impl Delay {
    pub fn new(fallback_latency_ms: i32, latency_safety_ms: i32) -> Self {
        Delay {
            fallback_latency_ms,
            latency_safety_ms,
            prev_timeout: 0,
            prev_delay: 0.0,
        }
    }

    pub fn next_delay_us(&mut self, measured_latency_ms: i32, timeout_ms: i32) -> i64 {
        let curr_target_latency_ms = timeout_ms - self.latency_safety_ms;
        if measured_latency_ms == 0 {
            self.prev_delay = curr_target_latency_ms as f64 - self.fallback_latency_ms as f64;
        } else {
            let prev_target_latency_ms = self.prev_timeout - self.latency_safety_ms;
            let error_ms = prev_target_latency_ms as f64 - measured_latency_ms as f64;
            self.prev_delay += error_ms;
        }
        self.prev_timeout = timeout_ms;
        (self.prev_delay * 1000.0).round() as i64
    }
}
