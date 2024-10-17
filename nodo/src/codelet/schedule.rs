// Copyright 2024 by David Weikersdorfer. All rights reserved.

use crate::{
    codelet::{CodeletInstance, DynamicVise},
    prelude::{Codelet, Sequence},
};
use core::time::Duration;

/// A helper type to build a schedule
pub struct ScheduleBuilder {
    pub name: String,
    pub thread_id: usize,
    pub sequences: Vec<Sequence>,
    pub max_step_count: Option<usize>,
    pub period: Option<Duration>,
}

impl ScheduleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            thread_id: 0,
            sequences: Vec::new(),
            max_step_count: None,
            period: None,
        }
    }

    #[must_use]
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = name.into();
        self
    }

    #[must_use]
    pub fn with_thread_id(mut self, thread_id: usize) -> Self {
        self.thread_id = thread_id;
        self
    }

    #[must_use]
    pub fn with_period(mut self, period: Duration) -> Self {
        self.period = Some(period);
        self
    }

    #[deprecated]
    #[must_use]
    pub fn with_max_step_count(mut self, max_step_count: usize) -> Self {
        self.max_step_count = Some(max_step_count);
        self
    }

    /// Add nodos to the schedule (builder style)
    #[must_use]
    pub fn with<A: Schedulable>(mut self, x: A) -> Self {
        x.schedule(&mut self);
        self
    }

    /// Add nodos to the schedule
    pub fn append<A: Schedulable>(&mut self, x: A) {
        x.schedule(self);
    }
}

/// Types implementing this trait can be scheduled using the schedule builder
pub trait Schedulable {
    fn schedule(self, sched: &mut ScheduleBuilder);
}

impl<C: Codelet + 'static> Schedulable for CodeletInstance<C> {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        sched.sequences.push(Sequence {
            name: "".into(),
            vises: vec![DynamicVise::new(self)],
            period: None,
        });
    }
}

impl Schedulable for Sequence {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        sched.sequences.push(self);
    }
}

impl<T1> Schedulable for (T1,)
where
    T1: Schedulable,
{
    fn schedule(self, sched: &mut ScheduleBuilder) {
        self.0.schedule(sched);
    }
}

impl<T1, T2> Schedulable for (T1, T2)
where
    T1: Schedulable,
    T2: Schedulable,
{
    fn schedule(self, sched: &mut ScheduleBuilder) {
        self.0.schedule(sched);
        self.1.schedule(sched);
    }
}

impl<T1, T2, T3> Schedulable for (T1, T2, T3)
where
    T1: Schedulable,
    T2: Schedulable,
    T3: Schedulable,
{
    fn schedule(self, sched: &mut ScheduleBuilder) {
        self.0.schedule(sched);
        self.1.schedule(sched);
        self.2.schedule(sched);
    }
}

impl<T1, T2, T3, T4> Schedulable for (T1, T2, T3, T4)
where
    T1: Schedulable,
    T2: Schedulable,
    T3: Schedulable,
    T4: Schedulable,
{
    fn schedule(self, sched: &mut ScheduleBuilder) {
        self.0.schedule(sched);
        self.1.schedule(sched);
        self.2.schedule(sched);
        self.3.schedule(sched);
    }
}

impl<A: Schedulable> Schedulable for Option<A> {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        if let Some(a) = self {
            a.schedule(sched);
        }
    }
}

impl<A: Schedulable> Schedulable for Box<A> {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        (*self).schedule(sched);
    }
}
