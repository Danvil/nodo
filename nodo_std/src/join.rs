// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::{channels::SyncResult, prelude::*};
use nodo_core::{Outcome, SUCCESS};

#[derive(Default)]
pub struct JoinConfig {
    pub input_count: usize,
}

/// Join has multiple input channels and a single output channel. All messages received on any
/// input channel are sent to the output channel. There is no particular guarantee on the order
/// of messages on the output channel.
pub struct Join<T>(PhantomData<T>);

impl<T: Send + Sync + Clone> Default for Join<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Send + Sync + Clone> Codelet for Join<T> {
    type Status = DefaultStatus;
    type Config = JoinConfig;
    type Rx = JoinRx<T>;
    type Tx = DoubleBufferTx<T>;

    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            JoinRx::new(cfg.input_count),
            DoubleBufferTx::new_auto_size(),
        )
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        for channel in rx.inputs.iter_mut() {
            tx.push_many(channel.drain(..))?;
        }
        SUCCESS
    }
}

pub struct JoinRx<T> {
    inputs: Vec<DoubleBufferRx<T>>,
}

impl<T> JoinRx<T> {
    pub fn new(count: usize) -> Self {
        Self {
            inputs: (0..count)
                .map(|_| DoubleBufferRx::new_auto_size())
                .collect(),
        }
    }

    /// Get the i-th input channel
    pub fn channel_mut(&mut self, index: usize) -> &mut DoubleBufferRx<T> {
        &mut self.inputs[index]
    }

    /// Add a new input channel and return it
    pub fn new_channel_mut(&mut self) -> &mut DoubleBufferRx<T> {
        self.inputs.push(DoubleBufferRx::new_auto_size());
        self.inputs.last_mut().unwrap()
    }
}

impl<T: Send + Sync> nodo::channels::RxBundle for JoinRx<T> {
    fn len(&self) -> usize {
        self.inputs.len()
    }

    fn name(&self, index: usize) -> String {
        if index < self.inputs.len() {
            format!("input_{index}")
        } else {
            panic!(
                "invalid index '{index}': number of inputs is {}",
                self.inputs.len()
            )
        }
    }

    fn sync_all(&mut self, results: &mut [SyncResult]) {
        for (i, channel) in self.inputs.iter_mut().enumerate() {
            results[i] = channel.sync()
        }
    }

    fn check_connection(&self) -> nodo::channels::ConnectionCheck {
        let mut cc = nodo::channels::ConnectionCheck::new(self.inputs.len());
        for (i, channel) in self.inputs.iter().enumerate() {
            cc.mark(i, channel.is_connected());
        }
        cc
    }
}
