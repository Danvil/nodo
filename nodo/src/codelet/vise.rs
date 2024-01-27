// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::codelet_instance::CodeletInstanceId;
use crate::codelet::Codelet;
use crate::codelet::CodeletExec;
use crate::codelet::CodeletInstance;
use crate::codelet::Statistics;
use crate::codelet::TaskClock;
use crate::codelet::Transition;
use crate::codelet::TransitionStatistics;
use nodo_core::Outcome;

/// Wrapper around a codelet with additional information
pub struct Vise {
    name: String,
    type_name: String,
    instance: Box<dyn CodeletExec>,
    statistics: Statistics,
}

impl Vise {
    pub fn new<C: Codelet + 'static>(mut instance: CodeletInstance<C>) -> Self {
        instance.is_scheduled = true; // TODO is this the right location?
        Self {
            name: instance.name.clone(),
            type_name: instance.type_name().to_string(),
            instance: Box::new(instance),
            statistics: Statistics::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn statistics(&self) -> &Statistics {
        &self.statistics
    }
}

impl CodeletExec for Vise {
    fn setup(&mut self, clock: TaskClock) {
        self.instance.setup(clock)
    }

    fn execute(&mut self, transition: Transition) -> Outcome {
        let _guard = TransitionStatisticsGuard::new(&mut self.statistics.transitions[transition]);
        self.instance.execute(transition)
    }
}

/// Helper type to capture statistics
struct TransitionStatisticsGuard<'a> {
    statistics: &'a mut TransitionStatistics,
}

impl<'a> TransitionStatisticsGuard<'a> {
    pub fn new(statistics: &'a mut TransitionStatistics) -> Self {
        statistics.begin();
        Self { statistics }
    }
}

impl<'a> Drop for TransitionStatisticsGuard<'a> {
    fn drop(&mut self) {
        self.statistics.end();
    }
}

impl<T: Codelet + 'static> From<CodeletInstance<T>> for Vise {
    fn from(other: CodeletInstance<T>) -> Vise {
        Vise::new(other)
    }
}
