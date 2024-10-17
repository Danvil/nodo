// Copyright 2024 by David Weikersdorfer. All rights reserved.

use crate::StateMachine;
use core::time::Duration;
use eyre::Result;
use nodo::codelet::{
    DynamicVise, Lifecycle, ScheduleBuilder, Statistics, TaskClocks, Transition, ViseTrait,
};
use nodo_core::{Report, *};
use std::{collections::HashMap, time::Instant};

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
    fn cycle(&mut self, transition: Transition) -> Result<OutcomeKind> {
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
