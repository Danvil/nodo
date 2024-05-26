// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet with a single TX which can be connected but which nevers publishes a message.
pub struct CallbackTx<T, F> {
    callback: F,
    marker: PhantomData<T>,
}

impl<T, F> CallbackTx<T, F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            marker: PhantomData,
        }
    }
}

impl<T, F> Codelet for CallbackTx<T, F>
where
    T: Send + Sync + Clone,
    F: FnMut() -> T + Send,
{
    type Config = ();
    type Rx = ();
    type Tx = DoubleBufferTx<T>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((), DoubleBufferTx::new(1))
    }

    fn step(&mut self, _: &Context<Self>, _: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        tx.push((self.callback)())?;
        SUCCESS
    }
}
