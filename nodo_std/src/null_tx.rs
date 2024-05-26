// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet with a single TX which can be connected but which nevers publishes a message.
pub struct NullTx<T>(PhantomData<T>);

impl<T> Default for NullTx<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Send + Sync + Clone> Codelet for NullTx<T> {
    type Config = ();
    type Rx = ();
    type Tx = DoubleBufferTx<T>;

    fn build_bundles(_cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((), DoubleBufferTx::new(0))
    }
}
