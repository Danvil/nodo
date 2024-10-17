// Copyright 2024 by David Weikersdorfer. All rights reserved.

use crate::{
    codelet::{
        vise::ViseTrait, CodeletInstance, DynamicVise, Lifecycle, StateMachine, Statistics,
        TaskClocks, Transition,
    },
    prelude::{Codelet, Sequence},
};
use core::time::Duration;
use nodo_core::{Report, *};
use std::{collections::HashMap, time::Instant};

/// A helper type to build a schedule
pub struct ScheduleBuilder {
    name: String,
    thread_id: usize,
    sequences: Vec<Sequence>,
    max_step_count: Option<usize>,
    period: Option<Duration>,
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

    #[deprecated(note = "use 'into' instead")]
    #[must_use]
    pub fn finalize(self) -> ScheduleExecutor {
        self.into()
    }
}

impl From<ScheduleBuilder> for ScheduleExecutor {
    fn from(builder: ScheduleBuilder) -> Self {
        ScheduleExecutor {
            name: builder.name,
            thread_id: builder.thread_id,
            sm: StateMachine::new(SequenceGroupExec::new(
                builder
                    .sequences
                    .into_iter()
                    .map(|seq| SequenceExec::new(seq.name, seq.period, seq.vises)),
            )),
            next_transition: Some(Transition::Start),
            max_step_count: builder.max_step_count,
            num_steps: 0,
            period: builder.period,
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

/// A schedule of codelets to be executed
#[derive(Debug)]
pub struct ScheduleExecutor {
    name: String,
    thread_id: usize,
    sm: StateMachine<SequenceGroupExec>,
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

    pub fn setup_task_clocks(&mut self, clocks: TaskClocks) {
        self.sm.inner_mut().setup_task_clocks(clocks);
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

/// A group of codelet sequences which are executed one after another
///
/// The group runs as long as any item in it is running.
pub(crate) struct SequenceGroupExec {
    items: Vec<SequenceExec>,
}

impl SequenceGroupExec {
    pub fn new<I: IntoIterator<Item = SequenceExec>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
        }
    }

    pub fn setup_task_clocks(&mut self, clocks: TaskClocks) {
        for item in self.items.iter_mut() {
            item.setup_task_clocks(clocks.clone());
        }
    }

    pub fn statistics(&self) -> HashMap<(String, String), Statistics> {
        let mut result = HashMap::new();
        for item in self.items.iter() {
            result.extend(item.statistics());
        }
        result
    }
}

impl Lifecycle for SequenceGroupExec {
    fn cycle(&mut self, transition: Transition) -> Outcome {
        let mut is_any_running = false;
        for item in self.items.iter_mut() {
            match item.cycle(transition)? {
                OutcomeKind::Skipped => {}
                OutcomeKind::Running => is_any_running = true,
            }
        }
        if is_any_running {
            RUNNING
        } else {
            SKIPPED
        }
    }
}

/// Executes a Sequence of nodos.
pub(crate) struct SequenceExec {
    name: String,
    period: Option<Duration>,
    items: Vec<StateMachine<DynamicVise>>,
}

impl SequenceExec {
    pub fn new<I: IntoIterator<Item = DynamicVise>>(
        name: String,
        period: Option<Duration>,
        vises: I,
    ) -> Self {
        Self {
            name,
            period,
            items: vises
                .into_iter()
                .map(|vise| StateMachine::new(vise))
                .collect(),
        }
    }

    pub fn setup_task_clocks(&mut self, clocks: TaskClocks) {
        for csm in self.items.iter_mut() {
            csm.inner_mut().setup_task_clocks(clocks.clone());
        }
    }

    pub fn statistics(&self) -> HashMap<(String, String), Statistics> {
        self.items
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

impl Lifecycle for SequenceExec {
    fn cycle(&mut self, transition: Transition) -> Outcome {
        let mut result = SequenceExecCycleResult::new();

        for csm in self.items.iter_mut() {
            match csm.transition(transition) {
                Err(err) => {
                    result.mark(csm.inner(), err.into());
                }
                Ok(_) => {}
            }
        }

        match result.into() {
            Some(err) => Err(err),
            None => RUNNING,
        }
    }
}

struct SequenceExecCycleResult {
    maybe: Option<SequenceExecCycleError>,
}

impl SequenceExecCycleResult {
    fn new() -> Self {
        SequenceExecCycleResult { maybe: None }
    }

    fn mark(&mut self, vise: &DynamicVise, error: Report) {
        if self.maybe.is_none() {
            self.maybe = Some(SequenceExecCycleError::new());
        }

        // SAFETY: `maybe` is cannot be None due to code above
        self.maybe.as_mut().unwrap().mark(vise, error);
    }
}

#[derive(thiserror::Error, Debug)]
#[error("SequenceExecCycleError({:?})", self.failures)]
struct SequenceExecCycleError {
    failures: Vec<(String, Report)>,
}

impl SequenceExecCycleError {
    fn new() -> Self {
        SequenceExecCycleError {
            failures: Vec::new(),
        }
    }

    fn mark(&mut self, vise: &DynamicVise, error: Report) {
        self.failures.push((vise.name().to_string(), error));
    }
}

impl From<SequenceExecCycleResult> for Option<eyre::Report> {
    fn from(value: SequenceExecCycleResult) -> Self {
        match value.maybe {
            Some(x) => Some(x.into()),
            None => None,
        }
    }
}
