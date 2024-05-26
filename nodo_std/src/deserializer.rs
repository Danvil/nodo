// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;
use nodo_core::BinaryFormat;
use nodo_core::SerializedMessage;

/// A codelet which serializes a message
pub struct Deserializer<T, BF> {
    format: BF,
    marker: PhantomData<T>,
}

impl<T, BF> Deserializer<T, BF> {
    pub fn new(format: BF) -> Self {
        Self {
            format,
            marker: PhantomData::default(),
        }
    }
}

impl<T, BF> Codelet for Deserializer<T, BF>
where
    T: Send + Sync + Clone,
    BF: Send + BinaryFormat<T>,
{
    type Config = ();
    type Rx = DoubleBufferRx<SerializedMessage>;
    type Tx = DoubleBufferTx<Message<T>>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new_auto_size(),
            DoubleBufferTx::new_auto_size(),
        )
    }

    fn step(&mut self, cx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        while let Some(message) = rx.try_pop() {
            tx.push(Message {
                seq: message.seq,
                stamp: Stamp {
                    acqtime: message.stamp.acqtime,
                    pubtime: cx.clock.step_time(),
                },
                value: self.format.deserialize(message.value.buffer)?,
            })?;
        }
        SUCCESS
    }
}
