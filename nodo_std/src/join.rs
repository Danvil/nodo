// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::channels::DoubleBufferRx;
use nodo::channels::DoubleBufferTx;
use nodo::channels::{Rx, Tx};
use nodo::codelet::Codelet;
use nodo::codelet::Context;
use nodo_core::Outcome;
use nodo_core::SUCCESS;

/// Join has multiple input channels and a single output channel. All messages received on any
/// input channel are sent to the output channel. There is no particular guarantee on the order
/// of messages on the output channel.
pub struct Join<T> {
    pd: PhantomData<T>,
}

impl<T: Send + Sync + Clone> Default for Join<T> {
    fn default() -> Self {
        Self {
            pd: PhantomData::default(),
        }
    }
}

#[derive(Default)]
pub struct JoinConfig {
    pub input_count: usize,
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

    fn sync(&mut self) {
        for channel in self.inputs.iter_mut() {
            channel.sync()
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

pub struct JoinTx<T> {
    pub output: DoubleBufferTx<T>,
}

impl<T: Send + Sync + Clone> nodo::channels::TxBundle for JoinTx<T> {
    fn name(&self, index: usize) -> String {
        if index != 0 {
            panic!("index must be 0");
        }
        "output".to_string()
    }

    fn flush(&mut self) -> Result<(), nodo::channels::MultiFlushError> {
        self.output
            .flush()
            .map_err(|e| nodo::channels::MultiFlushError(vec![(0, e)]))
    }

    fn check_connection(&self) -> nodo::channels::ConnectionCheck {
        let mut cc = nodo::channels::ConnectionCheck::new(1);
        cc.mark(0, self.output.is_connected());
        cc
    }
}

impl<T: Send + Sync + Clone> Codelet for Join<T> {
    type Config = JoinConfig;
    type Rx = JoinRx<T>;
    type Tx = JoinTx<T>;

    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            JoinRx::new(cfg.input_count),
            Self::Tx {
                output: DoubleBufferTx::new_auto_size(),
            },
        )
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        for channel in rx.inputs.iter_mut() {
            tx.output.push_many(channel.drain(..))?;
        }
        SUCCESS
    }
}
