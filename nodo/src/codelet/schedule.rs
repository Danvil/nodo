// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::CodeletInstance;
use crate::codelet::CodeletSequence;
use crate::codelet::DynamicVise;
use crate::codelet::StateMachine;
use crate::codelet::Statistics;
use crate::codelet::TaskClock;
use crate::codelet::Transition;
use crate::prelude::Codelet;
use nodo_core::*;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

/// A helper type to build a schedule
pub struct ScheduleBuilder {
    name: String,
    thread_id: usize,
    vises: Vec<DynamicVise>,
    max_step_count: Option<usize>,
    period: Option<Duration>,
}

impl ScheduleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            thread_id: 0,
            vises: Vec::new(),
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

    #[must_use]
    pub fn with_max_step_count(mut self, max_step_count: usize) -> Self {
        self.max_step_count = Some(max_step_count);
        self
    }

    #[must_use]
    pub fn with<A: Schedulable>(mut self, x: A) -> Self {
        x.schedule(&mut self);
        self
    }

    pub fn append<C: Codelet + 'static>(&mut self, instance: CodeletInstance<C>) {
        self.vises.push(DynamicVise::new(instance));
    }

    #[must_use]
    pub fn finalize(self) -> ScheduleExecutor {
        ScheduleExecutor {
            name: self.name,
            thread_id: self.thread_id,
            sm: StateMachine::new(CodeletSequence::new(self.vises)),
            next_transition: Some(Transition::Start),
            max_step_count: self.max_step_count,
            num_steps: 0,
            period: self.period,
            last_instant: None,
        }
    }
}

/// Types implementing this trait can be scheduled using the schedule builder
pub trait Schedulable {
    fn schedule(self, sched: &mut ScheduleBuilder);
}

impl<C: Codelet + 'static> Schedulable for CodeletInstance<C> {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        sched.append(self);
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

/// A schedule of codelets to be executed
#[derive(Debug)]
pub struct ScheduleExecutor {
    name: String,
    thread_id: usize,
    sm: StateMachine<CodeletSequence>,
    next_transition: Option<Transition>,
    max_step_count: Option<usize>,
    num_steps: usize,
    period: Option<Duration>,
    last_instant: Option<Instant>,
}

impl ScheduleExecutor {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn thread_id(&self) -> usize {
        self.thread_id
    }

    pub fn is_terminated(&self) -> bool {
        self.next_transition.is_none()
    }

    pub fn period(&self) -> Option<Duration> {
        self.period
    }

    pub fn last_instant(&self) -> Option<Instant> {
        self.last_instant
    }

    pub fn setup_task_clock(&mut self, clock: TaskClock) {
        self.sm.inner_mut().setup_task_clock(clock);
    }

    pub fn spin(&mut self) {
        let time_begin = Instant::now();
        self.last_instant = Some(time_begin);

        if self.next_transition.is_some() {
            if let Some(max_step_count) = self.max_step_count {
                if self.num_steps >= max_step_count {
                    self.next_transition = Some(Transition::Stop);
                }
            }
        }

        if let Some(transition) = self.next_transition {
            if transition == Transition::Step {
                self.num_steps += 1;
            }

            let result = self.sm.transition(transition);

            match result {
                Ok(OutcomeKind::Terminated) => {
                    self.next_transition = Some(Transition::Stop);
                }
                Ok(OutcomeKind::Running) | Ok(OutcomeKind::Skipped) => {
                    self.next_transition = match transition {
                        Transition::Start | Transition::Step | Transition::Resume => {
                            Some(Transition::Step)
                        }
                        Transition::Pause | Transition::Stop => None,
                    };
                }
                Err(err) => {
                    log::error!("Schedule {:?} error: {err:?}", self.name);
                    log::info!("Stopping schedule {:?}.", self.name);

                    self.next_transition = match transition {
                        Transition::Stop => None,
                        _ => Some(Transition::Stop),
                    };
                }
            }
        }
    }

    pub fn finalize(&mut self) {
        if self.sm.is_valid_request(Transition::Stop) {
            self.sm.transition(Transition::Stop).unwrap();
            self.next_transition = None;
        }
    }

    pub fn statistics(&self) -> HashMap<(String, String), Statistics> {
        self.sm.inner().statistics()
    }
}
