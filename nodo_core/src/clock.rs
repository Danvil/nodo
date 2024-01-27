// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::Timestamp;
use std::time::Instant;

const DEFAULT_CLOCK_ID: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockId(u64);

impl Default for ClockId {
    fn default() -> Self {
        ClockId(DEFAULT_CLOCK_ID)
    }
}

impl ClockId {
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl ClockId {
    pub fn is_default(&self) -> bool {
        self.0 == DEFAULT_CLOCK_ID
    }
}

pub trait Clock {
    fn id(&self) -> ClockId;
    fn now(&self) -> Timestamp;
}

#[derive(Clone)]
pub struct MonotonicClock {
    clock_id: ClockId,
    reference: Instant,
}

impl Clock for MonotonicClock {
    fn id(&self) -> ClockId {
        self.clock_id
    }

    fn now(&self) -> Timestamp {
        Timestamp::new(self.clock_id, self.reference.elapsed())
    }
}

impl MonotonicClock {
    pub fn new(clock_id: ClockId) -> Self {
        Self {
            clock_id,
            reference: Instant::now(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        MonotonicClock::new(ClockId::default())
    }
}
