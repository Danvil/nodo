// Copyright 2023 by David Weikersdorfer. All rights reserved.

use std::time::{Duration, Instant};

/// Sleeps for a certain duration with high accuracy potentially using a spin loop
pub fn accurate_sleep(duration: Duration) {
    accurate_sleep_impl(Instant::now() + duration, duration);
}

/// Sleeps up to a time instant with high accuracy potentially using a spin loop
pub fn accurate_sleep_until(target: Instant) {
    accurate_sleep_impl(target, target - Instant::now()); // Duration will wrap to 0
}

fn accurate_sleep_impl(target: Instant, duration: Duration) {
    const NATIVE_ACCURACY: Duration = Duration::from_millis(15); // TODO

    // native sleep for majority up to accuracy
    if duration > NATIVE_ACCURACY {
        let native_sleep_duration = duration - NATIVE_ACCURACY;
        std::thread::sleep(native_sleep_duration);
    }

    // spin the rest
    while Instant::now() <= target {
        std::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use crate::sleep::{accurate_sleep, accurate_sleep_until};
    use core::time::Duration;
    use std::time::Instant;

    #[test]
    fn test_accurate_sleep() {
        accurate_sleep(Duration::from_millis(100));
        accurate_sleep_until(Instant::now() + Duration::from_millis(100));
        accurate_sleep_until(Instant::now() - Duration::from_millis(100));
    }
}
