// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet with a single RX which calls a callback for every received message
pub struct CallbackRx<T, F> {
    callback: F,
    marker: PhantomData<T>,
}

impl<T, F> CallbackRx<T, F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            marker: PhantomData,
        }
    }
}

impl<T, F> Codelet for CallbackRx<T, F>
where
    T: Send + Sync,
    F: FnMut(T) + Send,
{
    type Config = ();
    type Rx = DoubleBufferRx<T>;
    type Tx = ();

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (DoubleBufferRx::new_auto_size(), ())
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        while let Some(msg) = rx.try_pop() {
            (self.callback)(msg)
        }
        SUCCESS
    }
}
