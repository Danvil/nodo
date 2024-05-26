// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::channels::DoubleBufferRx;
use nodo::channels::DoubleBufferTx;
use nodo::channels::{Rx, Tx};
use nodo::codelet::Codelet;
use nodo::codelet::Context;
use nodo_core::ensure;
use nodo_core::Outcome;
use nodo_core::SUCCESS;

/// A multiplexer has multiple input inputs and a single output channel. Messages received on
/// the selected input channel are send on the output channel and messages on other inputs are
/// discarded. The channel can be selected via a separate input channel.
pub struct Multiplexer<T> {
    selection: Option<usize>,
    pd: PhantomData<T>,
}

impl<T: Send + Sync + Clone> Default for Multiplexer<T> {
    fn default() -> Self {
        Self {
            selection: None,
            pd: PhantomData::default(),
        }
    }
}

#[derive(Clone)]
pub struct MultiplexerSelection(pub usize);

pub struct MultiplexerConfig {
    pub initial_input_count: usize,
    pub initial_selection: Option<usize>,
}

pub struct MultiplexerRx<T> {
    inputs: Vec<DoubleBufferRx<T>>,
    selection: DoubleBufferRx<MultiplexerSelection>,
}

impl<T> MultiplexerRx<T> {
    pub fn new(count: usize) -> Self {
        Self {
            inputs: (0..count)
                .map(|_| DoubleBufferRx::new_auto_size())
                .collect(),
            selection: DoubleBufferRx::new_latest(),
        }
    }

    /// Gets the selection channel
    pub fn selection_mut(&mut self) -> &mut DoubleBufferRx<MultiplexerSelection> {
        &mut self.selection
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

impl<T: Send + Sync> nodo::channels::RxBundle for MultiplexerRx<T> {
    fn name(&self, index: usize) -> String {
        if index < self.inputs.len() {
            format!("{index}")
        } else if index == self.inputs.len() {
            "selection".to_string()
        } else {
            panic!(
                "invalid index '{index}': number of inputs is {}",
                self.inputs.len()
            )
        }
    }

    fn sync(&mut self) {
        for channel in self.inputs.iter_mut() {
            channel.sync();
        }
        self.selection.sync();
    }

    fn check_connection(&self) -> nodo::channels::ConnectionCheck {
        let mut cc = nodo::channels::ConnectionCheck::new(self.inputs.len() + 1);
        for (i, channel) in self.inputs.iter().enumerate() {
            cc.mark(i, channel.is_connected());
        }
        cc.mark(self.inputs.len(), self.selection.is_connected());
        cc
    }
}

pub struct MultiplexerTx<T> {
    pub output: DoubleBufferTx<T>,
}

impl<T: Send + Sync + Clone> nodo::channels::TxBundle for MultiplexerTx<T> {
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

impl<T: Send + Sync + Clone> Codelet for Multiplexer<T> {
    type Config = MultiplexerConfig;
    type Rx = MultiplexerRx<T>;
    type Tx = MultiplexerTx<T>;

    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            MultiplexerRx::new(cfg.initial_input_count),
            Self::Tx {
                output: DoubleBufferTx::new_auto_size(),
            },
        )
    }

    fn start(&mut self, cx: &Context<Self>, rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        self.update_selection(cx.config.initial_selection, rx.inputs.len())?;
        Ok(())
    }

    fn step(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        // React to channel selection
        if let Some(MultiplexerSelection(selection)) = rx.selection.try_pop() {
            self.update_selection(Some(selection), rx.inputs.len())?;
        }

        // First forward messages from selected input
        if let Some(selection) = self.selection {
            tx.output.push_many(rx.inputs[selection].drain(..))?;
        }

        // Then discard all messages from other inputs
        for (i, channel) in rx.inputs.iter_mut().enumerate() {
            if Some(i) == self.selection {
                continue;
            }

            channel.drain(..);
        }

        SUCCESS
    }
}

impl<T> Multiplexer<T> {
    fn update_selection(&mut self, selection: Option<usize>, channel_count: usize) -> Outcome {
        if let Some(selection) = selection {
            ensure!(
                selection < channel_count,
                "invalid input channel {selection}. channel count: {}",
                channel_count
            );
            self.selection = Some(selection);
        } else {
            self.selection = None;
        }
        Ok(())
    }
}
