// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet which calls a callback for every received message
pub struct Sink<T, F> {
    callback: F,
    marker: PhantomData<T>,
}

impl<T, F> Sink<T, F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            marker: PhantomData,
        }
    }
}

impl<T, F> Codelet for Sink<T, F>
where
    T: Send + Sync,
    F: FnMut(T) -> Outcome + Send,
{
    type Status = DefaultStatus;
    type Config = ();
    type Rx = DoubleBufferRx<T>;
    type Tx = ();

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (DoubleBufferRx::new_auto_size(), ())
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        if rx.is_empty() {
            SKIPPED
        } else {
            while let Some(msg) = rx.try_pop() {
                (self.callback)(msg)?;
            }
            SUCCESS
        }
    }
}
