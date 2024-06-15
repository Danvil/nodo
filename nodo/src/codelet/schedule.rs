// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::CodeletExec;
use crate::codelet::StateMachine;
use crate::codelet::Statistics;
use crate::codelet::TaskClock;
use crate::codelet::Transition;
use crate::codelet::Vise;
use nodo_core::*;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

/// A helper type to build a schedule
pub struct ScheduleBuilder {
    codelets: Vec<Vise>,
    name: String,
    thread_id: usize,
    max_step_count: Option<usize>,
    period: Option<Duration>,
}

impl ScheduleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            thread_id: 0,
            codelets: Vec::new(),
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

    pub fn append<T: Into<Vise>>(&mut self, instance: T) {
        self.codelets.push(instance.into());
    }

    #[must_use]
    pub fn finalize(self) -> ScheduleExecutor {
        let cseq = CodeletSequence {
            items: self
                .codelets
                .into_iter()
                .map(|vise| StateMachine::new(vise))
                .collect(),
            has_error: false,
        };

        ScheduleExecutor {
            name: self.name,
            thread_id: self.thread_id,
            sm: StateMachine::new(cseq),
            is_terminated: false,
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

impl Schedulable for Vec<Vise> {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        for x in self.into_iter() {
            sched.append(x);
        }
    }
}

impl<T: Into<Vise>> Schedulable for T {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        sched.append(self.into());
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
    is_terminated: bool,
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
        self.is_terminated
    }

    pub fn period(&self) -> Option<Duration> {
        self.period
    }

    pub fn last_instant(&self) -> Option<Instant> {
        self.last_instant
    }

    pub fn setup(&mut self, clock: TaskClock) {
        self.sm.inner_mut().setup(clock);
    }

    pub fn spin(&mut self) {
        if self.is_terminated {
            return;
        }

        let time_begin = Instant::now();
        // if let Some(last) = self.last_instant {
        //     println!(
        //         "{}: period: {:0.3} ms",
        //         self.num_steps,
        //         1000.0 * (time_begin - last).as_secs_f64()
        //     );
        // }
        self.last_instant = Some(time_begin);

        let force_stop = if let Some(max_step_count) = self.max_step_count {
            self.num_steps >= max_step_count
        } else {
            false
        };

        if !force_stop && self.sm.is_valid_request(Transition::Start) {
            self.sm.transition(Transition::Start).unwrap();
        } else if !force_stop && self.sm.is_valid_request(Transition::Step) {
            self.sm.transition(Transition::Step).unwrap();
            self.num_steps += 1;
        } else if force_stop || self.sm.is_valid_request(Transition::Stop) {
            self.sm.transition(Transition::Stop).unwrap();
            self.is_terminated = true;
        }

        // let time_end = Instant::now();
        // println!(
        //     "{}: duration: {:0.3} ms",
        //     self.num_steps,
        //     1000.0 * (time_end - time_begin).as_secs_f64()
        // );
    }

    pub fn finalize(&mut self) {
        if self.is_terminated {
            return;
        }

        if self.sm.is_valid_request(Transition::Stop) {
            self.sm.transition(Transition::Stop).unwrap();
            self.is_terminated = true;
        }
    }

    pub fn statistics(&self) -> HashMap<(String, String), Statistics> {
        self.sm
            .inner()
            .items
            .iter()
            .map(|vice| {
                (
                    (
                        vice.inner().name().to_string(),
                        vice.inner().type_name().to_string(),
                    ),
                    vice.inner().statistics().clone(),
                )
            })
            .collect()
    }
}

struct CodeletSequence {
    items: Vec<StateMachine<Vise>>,
    has_error: bool,
}

impl CodeletExec for CodeletSequence {
    fn setup(&mut self, clock: TaskClock) {
        for csm in self.items.iter_mut() {
            csm.inner_mut().setup(clock.clone());
        }
    }

    fn execute(&mut self, transition: Transition) -> Outcome {
        let mut result = CodeletSequenceExecuteResult::new();

        let mut is_terminated = true;
        for csm in self.items.iter_mut() {
            match csm.transition(transition) {
                Err(err) => {
                    result.mark(csm.inner(), err.into());
                    self.has_error = true;
                }
                Ok(kind) => {
                    if !matches!(kind, OutcomeKind::Terminated) {
                        is_terminated = false;
                    }
                }
            }
        }

        match result.into() {
            Some(err) => Err(err),
            None => {
                if is_terminated {
                    TERMINATED
                } else {
                    RUNNING
                }
            }
        }
    }
}

struct CodeletSequenceExecuteResult {
    maybe: Option<CodeletSequenceExecuteError>,
}

impl CodeletSequenceExecuteResult {
    fn new() -> Self {
        CodeletSequenceExecuteResult { maybe: None }
    }

    fn mark(&mut self, vise: &Vise, error: Report) {
        if self.maybe.is_none() {
            self.maybe = Some(CodeletSequenceExecuteError::new());
        }
        if let Some(err) = self.maybe.as_mut() {
            err.mark(vise, error);
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("CodeletSequenceExecuteError({:?})", self.failures)]
struct CodeletSequenceExecuteError {
    failures: Vec<(String, Report)>,
}

impl CodeletSequenceExecuteError {
    fn new() -> Self {
        CodeletSequenceExecuteError {
            failures: Vec::new(),
        }
    }

    fn mark(&mut self, vise: &Vise, error: Report) {
        self.failures.push((vise.name().to_string(), error));
    }
}

impl From<CodeletSequenceExecuteResult> for Option<eyre::Report> {
    fn from(value: CodeletSequenceExecuteResult) -> Self {
        match value.maybe {
            Some(x) => Some(x.into()),
            None => None,
        }
    }
}
