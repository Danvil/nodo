// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use log::error;
use nodo::channels::DoubleBufferRx;
use nodo::channels::DoubleBufferTx;
use nodo::codelet::Codelet;
use nodo::codelet::Context;
use nodo_core::EyreResult;
use nodo_core::Outcome;
use nodo_core::ProtoSerializable;
use nodo_core::RecorderChannelId;
use nodo_core::SerializedMessage;
use nodo_core::Timestamp;
use nodo_core::WithAcqtime;
use nodo_core::SUCCESS;

/// A codelet which serializes a message
pub struct Serializer<T> {
    channel_id: RecorderChannelId,
    sequence: u32,
    pd: PhantomData<T>,
}

impl<T> Serializer<T> {
    pub(crate) fn new(channel_id: RecorderChannelId) -> Self {
        Self {
            channel_id,
            sequence: 0,
            pd: PhantomData::default(),
        }
    }
}

impl<T> Codelet for Serializer<T>
where
    T: Send + Sync + WithAcqtime + ProtoSerializable,
{
    type Config = ();
    type Rx = (DoubleBufferRx<T>,);
    type Tx = (DoubleBufferTx<SerializedMessage>,);

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            // record all messages
            (DoubleBufferRx::new_auto_size(),),
            (DoubleBufferTx::new_auto_size(),),
        )
    }

    fn step(&mut self, cx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        while let Some(message) = rx.0.try_recv() {
            // Sequence number is increased first independent of message processing success. Thus
            // it is visible in the recorded log if messages are missing.
            self.sequence += 1;

            match self.send_one(message, cx.clock.step_time(), &mut tx.0) {
                Ok(()) => {}
                Err(err) => error!("error serializing message: {err:?}"),
            }
        }
        SUCCESS
    }
}

impl<T> Serializer<T>
where
    T: Send + Sync + WithAcqtime + ProtoSerializable,
{
    /// Serialize a message and send it out
    fn send_one(
        &mut self,
        message: T,
        pubtime: Timestamp,
        tx: &mut DoubleBufferTx<SerializedMessage>,
    ) -> EyreResult<()> {
        tx.send(SerializedMessage {
            channel_id: self.channel_id,
            sequence: self.sequence - 1,
            acqtime: *message.acqtime(),
            pubtime: pubtime,
            buffer: message.into_proto()?,
        })?;
        Ok(())
    }
}
