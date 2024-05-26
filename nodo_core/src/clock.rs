// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::Timestamp;
use core::cell::RefCell;
use core::marker::PhantomData;
use std::sync::Arc;
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

#[derive(Clone)]
pub struct MonotonicClock<M> {
    reference: Instant,
    _marker: PhantomData<M>,
}

impl<M> Clock<M> for MonotonicClock<M> {
    fn now(&self) -> Timestamp<M> {
        Timestamp::new(self.reference.elapsed())
    }
}

impl<M> MonotonicClock<M> {
    pub fn new() -> Self {
        Self {
            reference: Instant::now(),
            _marker: PhantomData,
        }
    }
}

impl<M> Default for MonotonicClock<M> {
    fn default() -> Self {
        MonotonicClock::new()
    }
}

#[derive(Clone)]
pub struct SharedMonotonicClock<M>(Arc<RefCell<MonotonicClock<M>>>);

impl<M> SharedMonotonicClock<M> {
    pub fn new() -> Self {
        Self(Arc::new(RefCell::new(MonotonicClock::new())))
    }

    pub fn now(&self) -> Timestamp<M> {
        self.0.borrow().now()
    }
}
