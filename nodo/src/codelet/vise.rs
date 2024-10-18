// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::{
    Codelet, CodeletInstance, CodeletStatus, Lifecycle, Statistics, TaskClocks, Transition,
};
use eyre::Result;
use nodo_core::{DefaultStatus, OutcomeKind};

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
    fn cycle(&mut self, transition: Transition) -> Result<OutcomeKind> {
        let stats = &mut self.statistics.transitions[transition];
        stats.begin();

        let outcome = self.instance.cycle(transition)?;

        let skipped = outcome == OutcomeKind::Skipped;
        stats.end(skipped);

        Ok(outcome)
    }
}

pub trait ViseTrait: Send + Lifecycle {
    /// The name of the codelet instance
    fn name(&self) -> &str;

    /// The typename of the codelet used by this instance
    fn type_name(&self) -> &str;

    /// Gets the status as a string and the corresponding simplified status
    fn status(&self) -> Option<(String, DefaultStatus)>;

    /// Called once at the beginning to setup the clock
    fn setup_task_clocks(&mut self, clocks: TaskClocks);

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

    fn status(&self) -> Option<(String, DefaultStatus)> {
        self.instance
            .status
            .as_ref()
            .map(|s| (s.label().to_string(), s.as_default_status()))
    }

    fn setup_task_clocks(&mut self, clocks: TaskClocks) {
        self.instance.clocks = Some(clocks);
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

    fn status(&self) -> Option<(String, DefaultStatus)> {
        self.0.status()
    }

    fn setup_task_clocks(&mut self, clocks: TaskClocks) {
        self.0.setup_task_clocks(clocks);
    }

    fn statistics(&self) -> &Statistics {
        self.0.statistics()
    }
}

impl Lifecycle for DynamicVise {
    fn cycle(&mut self, transition: Transition) -> Result<OutcomeKind> {
        self.0.cycle(transition)
    }
}
