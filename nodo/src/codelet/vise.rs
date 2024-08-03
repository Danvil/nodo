// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::Codelet;
use crate::codelet::CodeletInstance;
use crate::codelet::Lifecycle;
use crate::codelet::Statistics;
use crate::codelet::TaskClock;
use crate::codelet::Transition;
use nodo_core::Outcome;
use nodo_core::OutcomeKind;

/// Wrapper around a codelet with additional information
pub struct Vise<C: Codelet> {
    instance: CodeletInstance<C>,
    statistics: Statistics,
}

impl<C: Codelet> Vise<C> {
    pub fn new(mut instance: CodeletInstance<C>) -> Self {
        instance.is_scheduled = true; // TODO is this the right location?
        Self {
            instance,
            statistics: Statistics::new(),
        }
    }

    pub fn statistics(&self) -> &Statistics {
        &self.statistics
    }
}

impl<C: Codelet> Lifecycle for Vise<C> {
    fn cycle(&mut self, transition: Transition) -> Outcome {
        let stats = &mut self.statistics.transitions[transition];
        stats.begin();

        let outcome = self.instance.cycle(transition);

        let skipped = matches!(outcome, Ok(OutcomeKind::Skipped));
        stats.end(skipped);

        return outcome;
    }
}

pub trait ViseTrait: Send + Lifecycle {
    /// The name of the codelet instance
    fn name(&self) -> &str;

    /// The typename of the codelet used by this instance
    fn type_name(&self) -> &str;

    /// Called once at the beginning to setup the clock
    fn setup_task_clock(&mut self, clock: TaskClock);

    /// Get instantce statistics
    fn statistics(&self) -> &Statistics;
}

impl<C: Codelet> ViseTrait for Vise<C> {
    fn name(&self) -> &str {
        &self.instance.name
    }

    fn type_name(&self) -> &str {
        self.instance.type_name()
    }

    fn setup_task_clock(&mut self, clock: TaskClock) {
        self.instance.clock = Some(clock);
    }

    fn statistics(&self) -> &Statistics {
        &self.statistics
    }
}

pub struct DynamicVise(pub(crate) Box<dyn ViseTrait>);

impl DynamicVise {
    pub fn new<C: Codelet + 'static>(instance: CodeletInstance<C>) -> Self {
        Self(Box::new(Vise::new(instance)))
    }
}

impl ViseTrait for DynamicVise {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn type_name(&self) -> &str {
        self.0.type_name()
    }

    fn setup_task_clock(&mut self, clock: TaskClock) {
        self.0.setup_task_clock(clock);
    }

    fn statistics(&self) -> &Statistics {
        self.0.statistics()
    }
}

impl Lifecycle for DynamicVise {
    fn cycle(&mut self, transition: Transition) -> Outcome {
        self.0.cycle(transition)
    }
}
