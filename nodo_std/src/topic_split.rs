// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::channels::FlushError;
use nodo::channels::MultiFlushError;
use nodo::codelet::Context;
use nodo::prelude::*;
use nodo_core::Topic;
use nodo_core::WithTopic;

/// Reroutes 'WithTopic' messages based on their topic to the right receiver.
pub struct TopicSplit<T> {
    marker: PhantomData<T>,
}

impl<T: Send + Sync + Clone> Default for TopicSplit<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData::default(),
        }
    }
}

impl<T: Send + Sync + Clone> Codelet for TopicSplit<T> {
    type Config = ();
    type Rx = DoubleBufferRx<Message<WithTopic<T>>>;
    type Tx = TopicSplitTx<Message<T>>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (DoubleBufferRx::new_auto_size(), TopicSplitTx::default())
    }

    fn step(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        for msg in rx.drain(..) {
            if let Some(tx) = tx.find_by_topic(&msg.value.topic) {
                tx.push(msg.map(|WithTopic { value, .. }| value))?;
            }
        }

        SUCCESS
    }
}

pub struct TopicSplitTx<T> {
    pub channels: Vec<(Topic, DoubleBufferTx<T>)>,
}

impl<T> Default for TopicSplitTx<T> {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
        }
    }
}

impl<T> TopicSplitTx<T> {
    /// Finds TX by topic
    pub fn find_by_topic(&mut self, needle: &Topic) -> Option<&mut DoubleBufferTx<T>> {
        self.channels
            .iter_mut()
            .find(|(key, _)| key == needle)
            .map(|(_, value)| value)
    }

    /// Add a new input channel and return it
    pub fn add(&mut self, topic: Topic) -> &mut DoubleBufferTx<T> {
        self.channels.push((topic, DoubleBufferTx::new_auto_size()));
        &mut self.channels.last_mut().unwrap().1
    }
}

impl<T: Send + Sync + Clone> nodo::channels::TxBundle for TopicSplitTx<T> {
    fn name(&self, index: usize) -> String {
        (&self.channels[index].0).into()
    }

    fn flush_all(&mut self) -> Result<(), nodo::channels::MultiFlushError> {
        let errs: Vec<(usize, FlushError)> = self
            .channels
            .iter_mut()
            .map(|(_, tx)| tx.flush())
            .enumerate()
            .filter_map(|(i, res)| {
                if let Some(err) = res.err() {
                    Some((i, err))
                } else {
                    None
                }
            })
            .collect();
        if errs.is_empty() {
            Ok(())
        } else {
            Err(MultiFlushError(errs))
        }
    }

    fn check_connection(&self) -> nodo::channels::ConnectionCheck {
        let mut cc = nodo::channels::ConnectionCheck::new(self.channels.len());
        for (i, channel) in self.channels.iter().enumerate() {
            cc.mark(i, channel.1.is_connected());
        }
        cc
    }
}
