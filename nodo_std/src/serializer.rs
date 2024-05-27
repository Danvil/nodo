// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;
use nodo_core::BinaryFormat;
use nodo_core::SerializedMessage;

/// A codelet which serializes a message
pub struct Serializer<T, BF> {
    format: BF,
    marker: PhantomData<T>,
}

pub struct SerializerConfig {
    /// Maximum number of messages which can be queued before messages are dropped.
    pub queue_size: usize,
}

impl Default for SerializerConfig {
    fn default() -> Self {
        Self { queue_size: 10 }
    }
}

impl<T, BF> Serializer<T, BF> {
    pub fn new(format: BF) -> Self {
        Self {
            format,
            marker: PhantomData::default(),
        }
    }
}

impl<T, BF> Codelet for Serializer<T, BF>
where
    T: Send + Sync,
    BF: Send + BinaryFormat<T>,
{
    type Config = SerializerConfig;
    type Rx = DoubleBufferRx<Message<T>>;
    type Tx = DoubleBufferTx<Message<Vec<u8>>>;

    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new(
                OverflowPolicy::Forget(cfg.queue_size),
                RetentionPolicy::Keep,
            ),
            DoubleBufferTx::new(cfg.queue_size),
        )
    }

    fn step(&mut self, cx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        while let Some(message) = rx.try_pop() {
            tx.push(SerializedMessage {
                seq: message.seq,
                stamp: Stamp {
                    acqtime: message.stamp.acqtime,
                    pubtime: cx.clock.step_time(),
                },
                value: self.format.serialize(&message.value)?,
            })?;
        }
        SUCCESS
    }
}
