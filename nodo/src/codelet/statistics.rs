// Copyright 2024 by David Weikersdorfer. All rights reserved.

use crate::codelet::TransitionMap;
use core::time::Duration;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub transitions: TransitionMap<TransitionStatistics>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TransitionStatistics {
    pub duration: CountTotal,
    pub period: CountTotal,
    pub skipped_count: u64,

    #[serde(skip)]
    last_exec_begin: Option<Instant>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CountTotal {
    count: u64,
    total: Duration,
    limits: (Duration, Duration),
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            transitions: TransitionMap::default(),
        }
    }
}

impl TransitionStatistics {
    pub fn new() -> Self {
        Self {
            duration: CountTotal::default(),
            period: CountTotal::default(),
            skipped_count: 0,
            last_exec_begin: None,
        }
    }

    /// Percentage of steps which were skipped
    pub fn skip_percent(&self) -> f32 {
        let total = self.skipped_count + self.duration.count;
        if total == 0 {
            0.
        } else {
            self.skipped_count as f32 / total as f32
        }
    }

    pub fn begin(&mut self) {
        let now = Instant::now();

        if let Some(last_exec) = self.last_exec_begin {
            self.period.push(now - last_exec);
        }

        self.last_exec_begin = Some(now);
    }

    pub fn end(&mut self, skipped: bool) {
        if skipped {
            self.skipped_count += 1;
        } else {
            self.duration.push(
                Instant::now()
                    - self
                        .last_exec_begin
                        .expect("end() must be called after begin()"),
            );
        }
    }
}

impl CountTotal {
    pub fn push(&mut self, dt: Duration) {
        self.count += 1;
        self.total += dt;
        self.limits = if self.count == 1 {
            (dt, dt)
        } else {
            (self.limits.0.min(dt), self.limits.1.max(dt))
        };
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn total(&self) -> Duration {
        self.total
    }

    pub fn average_ms(&self) -> Option<f32> {
        if self.count <= 0 {
            None
        } else {
            Some(self.total.as_secs_f32() * 1000.0 / (self.count as f32))
        }
    }

    pub fn min_ms(&self) -> Option<f32> {
        if self.count <= 0 {
            None
        } else {
            Some(self.limits.0.as_secs_f32() * 1000.0)
        }
    }

    pub fn max_ms(&self) -> Option<f32> {
        if self.count <= 0 {
            None
        } else {
            Some(self.limits.1.as_secs_f32() * 1000.0)
        }
    }
}
