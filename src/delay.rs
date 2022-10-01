#[derive(Debug, Clone)]
pub struct Delay {
    pub fallback_latency_ms: i32,
    pub latency_safety_ms: i32,
    pub prev_timeout_ms: i32,
    pub prev_delay_ms: f64,
}

impl Delay {
    pub fn new(fallback_latency_ms: i32, latency_safety_ms: i32) -> Self {
        Delay {
            fallback_latency_ms,
            latency_safety_ms,
            prev_timeout_ms: 0,
            prev_delay_ms: 0.0,
        }
    }

    pub fn next_delay_us(&mut self, measured_latency_ms: i32, timeout_ms: i32) -> i64 {
        if measured_latency_ms == 0 {
            self.prev_delay_ms = (timeout_ms - self.latency_safety_ms) as f64 - self.fallback_latency_ms as f64;
        } else {
            let timeout_diff_ms = (timeout_ms - self.prev_timeout_ms) as f64;
            let prev_target_latency_ms = self.prev_timeout_ms - self.latency_safety_ms;
            let error_ms = prev_target_latency_ms as f64 - measured_latency_ms as f64;
            self.prev_delay_ms += error_ms + timeout_diff_ms;
        }
        self.prev_timeout_ms = timeout_ms;
        (self.prev_delay_ms * 1000.0).round() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay() {
        let mut delay = Delay::new(10, 5);

        assert_eq!(delay.next_delay_us(0, 200), 185000);
        assert_eq!(delay.next_delay_us(200, 200), 180000);
        assert_eq!(delay.next_delay_us(195, 200), 180000);
        assert_eq!(delay.next_delay_us(195, 200), 180000);
        assert_eq!(delay.next_delay_us(195, 190), 170000);
        assert_eq!(delay.next_delay_us(185, 190), 170000);
        assert_eq!(delay.next_delay_us(190, 210), 185000);
    }
}
