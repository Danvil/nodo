// Copyright 2023 by David Weikersdorfer. All rights reserved.

use nodo_core::Clock;
use nodo_core::MonotonicClock;
use nodo_core::Timestamp;

#[derive(Clone)]
pub struct TaskClock {
    clock: MonotonicClock,
    last: Timestamp,
    dt: f32,
}

impl TaskClock {
    pub fn from(clock: MonotonicClock) -> Self {
        let last = clock.now();
        Self {
            clock,
            last,
            dt: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.last = self.clock.now();
    }

    pub fn step(&mut self) {
        let now = self.clock.now();
        let dt = now.elapsed_since_as_secs_f32(&self.last);
        self.last = now;
        self.dt = dt;
    }

    /// Time when the current step started. `step_time` is set at the beginning of start/step/stop
    /// functions and stays constant throughout the current step. Use `real_time` for a continuously
    /// updating time.
    pub fn step_time(&self) -> Timestamp {
        self.last
    }

    /// The current time from the default clock. `real_time` changes during start/step/stop
    /// functions. Use `step_time` for a timestep which remains constant during those functions.
    pub fn real_time(&self) -> Timestamp {
        self.clock.now()
    }

    /// Time elapsed in seconds since last step.
    // TODO Use Duration and introduce `dt_seconds_f32` (?).
    pub fn dt(&self) -> f32 {
        self.dt
    }
}
