// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;
use nodo_core::BinaryFormat;
use nodo_core::RecorderChannelId;
use nodo_core::SerializedMessage;
use nodo_core::SerializedValue;

/// A codelet which serializes a message
pub struct Serializer<T, BF> {
    channel_id: RecorderChannelId,
    format: BF,
    marker: PhantomData<T>,
}

impl<T, BF> Serializer<T, BF> {
    pub fn new(channel_id: RecorderChannelId, format: BF) -> Self {
        Self {
            channel_id,
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
    type Config = ();
    type Rx = DoubleBufferRx<Message<T>>;
    type Tx = DoubleBufferTx<SerializedMessage>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new_auto_size(),
            DoubleBufferTx::new_auto_size(),
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
                value: SerializedValue {
                    channel_id: self.channel_id,
                    buffer: self.format.serialize(&message.value)?,
                },
            })?;
        }
        SUCCESS
    }
}
