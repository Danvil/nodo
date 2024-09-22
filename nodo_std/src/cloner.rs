// Copyright 2023 by David Weikersdorfer. All rights reserved.

use nodo::prelude::*;

/// Publishes a clone of a value each frame
pub struct Cloner<T> {
    blueprint: T,
    max_count: Option<usize>,
    count: usize,
}

impl<T> Cloner<T> {
    pub fn new_unlimited(blueprint: T) -> Self {
        Self {
            blueprint,
            max_count: None,
            count: 0,
        }
    }

    pub fn new_limited(blueprint: T, max_count: usize) -> Self {
        Self {
            blueprint,
            max_count: Some(max_count),
            count: 0,
        }
    }
}

impl<T: Clone + Send + Sync> Codelet for Cloner<T> {
    type Config = ();
    type Rx = ();
    type Tx = DoubleBufferTx<Message<T>>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((), DoubleBufferTx::new(1))
    }

    fn step(&mut self, cx: &Context<Self>, _: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        if let Some(max_count) = self.max_count {
            if self.count >= max_count {
                return SKIPPED;
            }
        }

        tx.push(Message {
            seq: 0,
            stamp: Stamp {
                acqtime: cx.clocks.sys_mono.now(),
                pubtime: cx.clocks.app_mono.now(),
            },
            value: self.blueprint.clone(),
        })?;

        self.count += 1;

        SUCCESS
    }
}
