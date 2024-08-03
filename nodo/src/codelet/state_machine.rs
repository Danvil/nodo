// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::Transition;
use core::fmt::Debug;
use core::fmt::Formatter;
use nodo_core::Outcome;
use nodo_core::OutcomeKind;
use nodo_core::Report;

pub trait Lifecycle {
    /// Applies a lifecycel change
    fn cycle(&mut self, transition: Transition) -> Outcome;
}

/// Possible states of codelets
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    /// Codelet is not started. The codelet can be started with the start transition
    Inactive,

    /// Codelet is started. The codelet can be stepped, paused or stopped.
    Started,

    /// Codelet is paused. Operation can be resumed with the resume transition. It is also possible
    /// to stop the codelet.
    Paused,
}

impl State {
    /// The next state after a successful state transition
    pub fn transition(self, request: Transition) -> Option<State> {
        match (self, request) {
            (State::Started, Transition::Stop) | (State::Paused, Transition::Stop) => {
                Some(State::Inactive)
            }
            (State::Inactive, Transition::Start)
            | (State::Started, Transition::Step)
            | (State::Paused, Transition::Resume) => Some(State::Started),
            (State::Started, Transition::Pause) => Some(State::Paused),
            (_, _) => None,
        }
    }
}

/// State machine which oversees correct codelet state transitions
pub struct StateMachine<C> {
    inner: C,
    state: State,
    has_error: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum TransitionError {
    /// Transition is not valid in the current state
    #[error("invalid transition {0:?} -> {1:?}")]
    InvalidTransition(State, Transition),

    /// Codelet transition function returned failure
    #[error("execution failed [{0:?}]: {1:?}")]
    ExecutionFailure(Transition, Report),
}

impl<C> StateMachine<C> {
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            state: State::Inactive,
            has_error: false,
        }
    }

    pub fn inner(&self) -> &C {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    pub fn is_valid_request(&self, request: Transition) -> bool {
        self.state.transition(request).is_some()
    }

    pub fn transition(&mut self, transition: Transition) -> Result<OutcomeKind, TransitionError>
    where
        C: Lifecycle,
    {
        if let Some(next_state) = self.state.transition(transition) {
            match self.inner.cycle(transition) {
                Ok(kind) => {
                    self.state = next_state;
                    return Ok(kind);
                }
                Err(err) => {
                    self.has_error = true;
                    return Err(TransitionError::ExecutionFailure(transition, err));
                }
            }
        } else {
            Err(TransitionError::InvalidTransition(self.state, transition))
        }
    }
}

impl<C> Debug for StateMachine<C> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.debug_struct("StateMachine")
            .field("inner", &"()")
            .field("state", &self.state)
            .field("has_error", &self.has_error)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::codelet::*;

    #[test]
    fn state_transition() {
        assert_eq!(
            State::Inactive.transition(Transition::Start),
            Some(State::Started)
        );
        assert_eq!(
            State::Started.transition(Transition::Step),
            Some(State::Started)
        );
        assert_eq!(
            State::Started.transition(Transition::Stop),
            Some(State::Inactive)
        );
    }
}
