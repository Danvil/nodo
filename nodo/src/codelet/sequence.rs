use crate::codelet::*;
use crate::prelude::*;
use nodo_core::Report;
use std::collections::HashMap;

/// A sequence of executables
///
/// The sequence fails/succeeds if any of its elements fails/succeeds.
pub struct CodeletSequence {
    items: Vec<StateMachine<DynamicVise>>,
}

impl CodeletSequence {
    pub fn new<I: IntoIterator<Item = DynamicVise>>(vises: I) -> Self {
        Self {
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

impl Lifecycle for CodeletSequence {
    fn cycle(&mut self, transition: Transition) -> Outcome {
        let mut result = CodeletSequenceExecuteResult::new();

        let mut is_terminated = false;

        for csm in self.items.iter_mut() {
            match csm.transition(transition) {
                Err(err) => {
                    result.mark(csm.inner(), err.into());
                }
                Ok(OutcomeKind::Terminated) => {
                    is_terminated = true;
                }
                Ok(_) => {}
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

    fn mark(&mut self, vise: &DynamicVise, error: Report) {
        if self.maybe.is_none() {
            self.maybe = Some(CodeletSequenceExecuteError::new());
        }

        // SAFETY: `maybe` is cannot be None due to code above
        self.maybe.as_mut().unwrap().mark(vise, error);
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

    fn mark(&mut self, vise: &DynamicVise, error: Report) {
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
