// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::channels::DoubleBufferRx;
use nodo::channels::DoubleBufferTx;
use nodo::codelet::Codelet;
use nodo::codelet::Context;
use nodo_core::Outcome;
use nodo_core::SUCCESS;

/// A codelet with a single RX which can be connected but which ignores all received messages.
pub struct NullRx<T> {
    pd: PhantomData<T>,
}

impl<T> Default for NullRx<T> {
    fn default() -> Self {
        Self {
            pd: PhantomData::default(),
        }
    }
}

impl<T: Send + Sync + Clone> Codelet for NullRx<T> {
    type Config = ();
    type Rx = (DoubleBufferRx<T>,);
    type Tx = ();

    fn build_bundles(_cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((DoubleBufferRx::new_auto_size(),), ())
    }

    fn step(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        rx.0.drain(..);
        SUCCESS
    }
}

/// A codelet with a single TX which can be connected but which nevers publishes a message.
pub struct NullTx<T> {
    pd: PhantomData<T>,
}

impl<T> Default for NullTx<T> {
    fn default() -> Self {
        Self {
            pd: PhantomData::default(),
        }
    }
}

impl<T: Send + Sync + Clone> Codelet for NullTx<T> {
    type Config = ();
    type Rx = ();
    type Tx = (DoubleBufferTx<T>,);

    fn build_bundles(_cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((), (DoubleBufferTx::new(0),))
    }
}
