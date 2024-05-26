// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet with a single RX which can be connected but which ignores all received messages.
pub struct NullRx<T>(PhantomData<T>);

impl<T> Default for NullRx<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Send + Sync + Clone> Codelet for NullRx<T> {
    type Config = ();
    type Rx = DoubleBufferRx<T>;
    type Tx = ();

    fn build_bundles(_cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (DoubleBufferRx::new_auto_size(), ())
    }

    fn step(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        rx.drain(..);
        SUCCESS
    }
}
