// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::ClockId;
use core::cmp::Ordering;
use core::ops::Add;
use core::time::Duration;

/// A timestamp from a specific clock
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timestamp {
    clock_id: ClockId,
    elapsed: Duration,
}

impl Default for Timestamp {
    fn default() -> Self {
        Self {
            clock_id: ClockId::default(),
            elapsed: Duration::from_secs_f32(0.0),
        }
    }
}

impl Timestamp {
    pub fn new(clock_id: ClockId, elapsed: Duration) -> Self {
        Self { clock_id, elapsed }
    }

    pub fn from_secs_f32(clock_id: ClockId, secs: f32) -> Self {
        Self {
            clock_id,
            elapsed: Duration::from_secs_f32(secs),
        }
    }

    pub fn as_secs_f32(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    pub fn as_secs_nanos(&self) -> (u64, u32) {
        (self.elapsed.as_secs(), self.elapsed.subsec_nanos())
    }

    pub fn as_nanos(&self) -> u128 {
        self.elapsed.as_nanos()
    }

    pub fn clock_id(&self) -> ClockId {
        self.clock_id
    }

    pub fn elapsed_since_as_secs_f32(&self, earlier: &Timestamp) -> f32 {
        assert!(self.elapsed >= earlier.elapsed);
        (self.elapsed - earlier.elapsed).as_secs_f32()
    }

    pub fn elapsed_since_as_secs_f32_or_zero(&self, earlier: &Timestamp) -> f32 {
        if self.elapsed < earlier.elapsed {
            0.0
        } else {
            (self.elapsed - earlier.elapsed).as_secs_f32()
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl Add<f32> for Timestamp {
    type Output = Timestamp;
    fn add(self, delta: f32) -> Timestamp {
        assert!(delta >= 0.0);
        Timestamp::from_secs_f32(self.clock_id(), self.as_secs_f32() + delta)
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, rhs: &Timestamp) -> Option<Ordering> {
        if self.clock_id() != rhs.clock_id() {
            None
        } else {
            self.elapsed.partial_cmp(&rhs.elapsed)
        }
    }
}

impl core::fmt::Display for Timestamp {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            fmt,
            "Timestamp({}|{:?})",
            self.clock_id.as_u64(),
            self.elapsed
        )
    }
}
