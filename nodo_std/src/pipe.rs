// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet which transforms messages with a callback.
pub struct Pipe<T, S, F> {
    callback: F,
    marker: PhantomData<(T, S)>,
}

impl<T, S, F> Pipe<T, S, F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            marker: PhantomData,
        }
    }
}

impl<T, S, F> Codelet for Pipe<T, S, F>
where
    T: Send + Sync,
    S: Clone + Send + Sync,
    F: FnMut(T) -> S + Send,
{
    type Config = ();
    type Rx = DoubleBufferRx<T>;
    type Tx = DoubleBufferTx<S>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new_auto_size(),
            DoubleBufferTx::new_auto_size(),
        )
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        while let Some(msg) = rx.try_pop() {
            tx.push((self.callback)(msg))?;
        }
        SUCCESS
    }
}
