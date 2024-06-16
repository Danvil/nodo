// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::codelet_instance::CodeletInstanceId;
use crate::codelet::Codelet;
use crate::codelet::CodeletExec;
use crate::codelet::CodeletInstance;
use crate::codelet::TaskClock;
use crate::codelet::Transition;
use crate::codelet::VertexStatistics;
use nodo_core::Outcome;
use nodo_core::OutcomeKind;

/// Wrapper around a codelet with additional information
pub struct Vise {
    name: String,
    type_name: String,
    instance: Box<dyn CodeletExec>,
    statistics: VertexStatistics,
}

impl Vise {
    pub fn new<C: Codelet + 'static>(mut instance: CodeletInstance<C>) -> Self {
        instance.is_scheduled = true; // TODO is this the right location?
        Self {
            name: instance.name.clone(),
            type_name: instance.type_name().to_string(),
            instance: Box::new(instance),
            statistics: VertexStatistics::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn statistics(&self) -> &VertexStatistics {
        &self.statistics
    }
}

impl CodeletExec for Vise {
    fn setup(&mut self, clock: TaskClock) {
        self.instance.setup(clock)
    }

    fn execute(&mut self, transition: Transition) -> Outcome {
        let stats = &mut self.statistics.0[transition];
        stats.begin();

        let outcome = self.instance.execute(transition);

        let skipped = matches!(outcome, Ok(OutcomeKind::Skipped));
        stats.end(skipped);

        return outcome;
    }
}

impl<T: Codelet + 'static> From<CodeletInstance<T>> for Vise {
    fn from(other: CodeletInstance<T>) -> Vise {
        Vise::new(other)
    }
}
