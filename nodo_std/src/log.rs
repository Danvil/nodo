// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::fmt::Debug;
use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet logs received messages with log crate
pub struct Log<T> {
    tag: String,
    marker: PhantomData<T>,
}

impl<T> Log<T> {
    pub fn new(tag: String) -> Self {
        Self {
            tag,
            marker: PhantomData,
        }
    }
}

impl<T> Default for Log<T> {
    fn default() -> Self {
        Self {
            tag: String::new(),
            marker: PhantomData,
        }
    }
}

impl<T: Send + Sync + Debug> Codelet for Log<T> {
    type Config = ();
    type Rx = DoubleBufferRx<T>;
    type Tx = ();

    fn build_bundles(_cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (DoubleBufferRx::new_auto_size(), ())
    }

    fn step(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        while let Ok(msg) = rx.pop() {
            log::info!("{}: {msg:?}", self.tag);
        }
        SUCCESS
    }
}
