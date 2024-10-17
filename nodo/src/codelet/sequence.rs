// Copyright 2024 by David Weikersdorfer. All rights reserved.

use crate::{
    codelet::{CodeletInstance, DynamicVise},
    prelude::Codelet,
};
use std::time::Duration;

/// A sequences of nodos (codelet instances) which are executed one after another in the given
/// order.
pub struct Sequence {
    pub name: String,
    pub period: Option<Duration>,
    pub vises: Vec<DynamicVise>,
}

impl Sequence {
    /// Create a new sequences
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            period: None,
            vises: Vec::new(),
        }
    }

    /// Give the sequences a name for debugging and reporting (builder style)
    #[must_use]
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = name.into();
        self
    }

    // TODO implement
    // #[must_use]
    // pub fn with_period(mut self, period: Duration) -> Self {
    //     self.period = Some(period);
    //     self
    // }

    /// Add nodos to the sequences (builder style)
    #[must_use]
    pub fn with<A: Sequenceable>(mut self, x: A) -> Self {
        x.append(&mut self);
        self
    }

    /// Add nodos to the sequences
    pub fn append<A: Sequenceable>(&mut self, x: A) {
        x.append(self);
    }
}

/// Types implementing this trait can be added to a sequence
pub trait Sequenceable {
    fn append(self, seq: &mut Sequence);

    fn into_sequence(self) -> Sequence
    where
        Self: Sized,
    {
        Sequence::new().with(self)
    }
}

impl<C: Codelet + 'static> Sequenceable for CodeletInstance<C> {
    fn append(self, seq: &mut Sequence) {
        seq.vises.push(DynamicVise::new(self));
    }
}

impl<T1> Sequenceable for (T1,)
where
    T1: Sequenceable,
{
    fn append(self, seq: &mut Sequence) {
        self.0.append(seq);
    }
}

impl<T1, T2> Sequenceable for (T1, T2)
where
    T1: Sequenceable,
    T2: Sequenceable,
{
    fn append(self, seq: &mut Sequence) {
        self.0.append(seq);
        self.1.append(seq);
    }
}

impl<T1, T2, T3> Sequenceable for (T1, T2, T3)
where
    T1: Sequenceable,
    T2: Sequenceable,
    T3: Sequenceable,
{
    fn append(self, seq: &mut Sequence) {
        self.0.append(seq);
        self.1.append(seq);
        self.2.append(seq);
    }
}

impl<T1, T2, T3, T4> Sequenceable for (T1, T2, T3, T4)
where
    T1: Sequenceable,
    T2: Sequenceable,
    T3: Sequenceable,
    T4: Sequenceable,
{
    fn append(self, seq: &mut Sequence) {
        self.0.append(seq);
        self.1.append(seq);
        self.2.append(seq);
        self.3.append(seq);
    }
}

impl<A: Sequenceable> Sequenceable for Option<A> {
    fn append(self, seq: &mut Sequence) {
        if let Some(a) = self {
            a.append(seq);
        }
    }
}

impl<A: Sequenceable> Sequenceable for Box<A> {
    fn append(self, seq: &mut Sequence) {
        (*self).append(seq);
    }
}
