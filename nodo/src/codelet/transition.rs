// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::{
    fmt,
    ops::{Index, IndexMut},
};

/// Codelet state transitions
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Transition {
    Start,
    Step,
    Stop,
    Pause,
    Resume,
}

impl Transition {
    pub const fn index(&self) -> usize {
        match self {
            Transition::Start => 0,
            Transition::Step => 1,
            Transition::Stop => 2,
            Transition::Pause => 3,
            Transition::Resume => 4,
        }
    }
}

/// Map of codelet transition function to custom data
#[derive(Default, Clone)]
pub struct TransitionMap<T>([T; 5]);

impl<T> Index<Transition> for TransitionMap<T> {
    type Output = T;

    fn index(&self, idx: Transition) -> &Self::Output {
        &self.0[idx.index()]
    }
}

impl<T> IndexMut<Transition> for TransitionMap<T> {
    fn index_mut(&mut self, idx: Transition) -> &mut Self::Output {
        &mut self.0[idx.index()]
    }
}

impl<T: fmt::Debug> fmt::Debug for TransitionMap<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.debug_struct("TransitionMap")
            .field("start", &self[Transition::Start])
            .field("step", &self[Transition::Step])
            .field("stop", &self[Transition::Stop])
            .field("pause", &self[Transition::Pause])
            .field("resume", &self[Transition::Resume])
            .finish()
    }
}
