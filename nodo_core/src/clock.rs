// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::Timestamp;
use core::marker::PhantomData;
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

pub trait Clock<M> {
    fn now(&self) -> Timestamp<M>;
}

/// A monotonic clock which starts when the application starts
#[derive(Clone)]
pub struct AppMonotonicClock<M> {
    reference: Instant,
    _marker: PhantomData<M>,
}

impl<M> Clock<M> for AppMonotonicClock<M> {
    fn now(&self) -> Timestamp<M> {
        Timestamp::new(self.reference.elapsed())
    }
}

impl<M> AppMonotonicClock<M> {
    pub fn new() -> Self {
        Self {
            reference: Instant::now(),
            _marker: PhantomData,
        }
    }
}

impl<M> Default for AppMonotonicClock<M> {
    fn default() -> Self {
        AppMonotonicClock::new()
    }
}

/// A monotonic clock which starts when the computer boots
///
/// TODO Currently uses nix::time::clock_gettime but it is unclear if that works under Windows and
///      or Mac.
#[derive(Clone)]
pub struct SysMonotonicClock<M> {
    _marker: PhantomData<M>,
}

impl<M> Clock<M> for SysMonotonicClock<M> {
    fn now(&self) -> Timestamp<M> {
        // SAFETY: According to the error values listed in the reference for clock_gettime
        //         (see https://man7.org/linux/man-pages/man3/clock_gettime.3.html)
        //         the get function should not return any errors for CLOCK_MONOTONIC.
        let time = nix::time::clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC).unwrap();
        Timestamp::new(std::time::Duration::from(time))
    }
}

impl<M> SysMonotonicClock<M> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<M> Default for SysMonotonicClock<M> {
    fn default() -> Self {
        SysMonotonicClock::new()
    }
}
