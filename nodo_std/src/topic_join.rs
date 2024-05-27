// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::channels::Rx;
use nodo::channels::RxBundle;
use nodo::codelet::Context;
use nodo::prelude::*;
use nodo_core::Topic;
use nodo_core::WithTopic;

/// Join has multiple input channels and a single output channel. All messages received on any
/// input channel are sent to the output channel. There is no particular guarantee on the order
/// of messages on the output channel.
pub struct TopicJoin<T> {
    marker: PhantomData<T>,
}

#[derive(Default)]
pub struct TopicJoinConfig;

impl<T> Default for TopicJoin<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData::default(),
        }
    }
}

impl<T> Codelet for TopicJoin<T>
where
    T: Clone + Send + Sync,
{
    type Config = TopicJoinConfig;
    type Rx = TopicJoinRx<Message<T>>;
    type Tx = DoubleBufferTx<Message<WithTopic<T>>>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (TopicJoinRx::default(), DoubleBufferTx::new_auto_size())
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        for (topic, channel) in rx.channels.iter_mut() {
            tx.push_many(channel.drain(..).map(|msg| {
                // FIXME should we re-stamp pubtime?
                msg.map(|value| WithTopic {
                    topic: topic.clone(),
                    value,
                })
            }))?;
        }
        SUCCESS
    }
}

pub struct TopicJoinRx<T> {
    channels: Vec<(Topic, DoubleBufferRx<T>)>,
}

impl<T> Default for TopicJoinRx<T> {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
        }
    }
}

impl<T> TopicJoinRx<T> {
    /// Finds RX by topic
    pub fn find_by_topic(&mut self, needle: &Topic) -> Option<&mut DoubleBufferRx<T>> {
        self.channels
            .iter_mut()
            .find(|(key, _)| key == needle)
            .map(|(_, value)| value)
    }

    /// Add a new input channel and return it
    pub fn add(&mut self, topic: Topic) -> &mut DoubleBufferRx<T> {
        self.channels.push((topic, DoubleBufferRx::new_auto_size()));
        &mut self.channels.last_mut().unwrap().1
    }
}

impl<T: Send + Sync> RxBundle for TopicJoinRx<T> {
    fn name(&self, index: usize) -> String {
        if index < self.channels.len() {
            format!("input_{index}")
        } else {
            panic!(
                "invalid index '{index}': number of channels is {}",
                self.channels.len()
            )
        }
    }

    fn sync_all(&mut self) {
        for channel in self.channels.iter_mut() {
            channel.1.sync()
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
