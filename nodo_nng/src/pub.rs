// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::NngPubSubHeader;
use log::error;
use log::info;
use log::trace;
use nng::Protocol;
use nng::Socket;
use nodo::prelude::*;
use nodo_core::SerializedMessage;

/// Codelet which receives serialized messages and writes them to MCAP
pub struct NngPub {
    socket: Option<Socket>,
    message_count: usize,
}

pub struct NngPubConfig {
    pub address: String,
    pub queue_size: usize,
}

impl Default for NngPub {
    fn default() -> Self {
        Self {
            socket: None,
            message_count: 0,
        }
    }
}

impl Codelet for NngPub {
    type Config = NngPubConfig;
    type Rx = DoubleBufferRx<SerializedMessage>;
    type Tx = ();

    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new(
                OverflowPolicy::Forget(cfg.queue_size),
                RetentionPolicy::Keep,
            ),
            (),
        )
    }

    fn start(&mut self, cx: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        info!("Opening PUB socket at '{}'..", cx.config.address);
        let socket = Socket::new(Protocol::Pub0)?;

        socket.pipe_notify(move |_, ev| {
            trace!("nng::socket::pipe_notify: {ev:?}");
        })?;

        let res = socket.listen(&cx.config.address);

        if let Err(err) = res {
            error!("   {err:?}");
            res?;
        }

        self.socket = Some(socket);

        SUCCESS
    }

    fn stop(&mut self, _cx: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        // SAFETY: guaranteed by start
        let socket = self.socket.take().unwrap();

        socket.close();

        SUCCESS
    }

    fn step(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        // SAFETY: guaranteed by start
        let socket = self.socket.as_mut().unwrap();

        while let Some(message) = rx.try_pop() {
            let header = NngPubSubHeader {
                magic: NngPubSubHeader::MAGIC,
                seq: message.seq.try_into()?,
                stamp: message.stamp,
                channel_id: message.value.channel_id,
                payload_checksum: NngPubSubHeader::CRC.checksum(&message.value.buffer),
            };

            let buffer = bincode::serialize(&header)?;
            socket.send(&buffer).map_err(|(_, err)| err)?;

            socket.send(&message.value.buffer).map_err(|(_, err)| err)?;

            self.message_count += 1;
        }

        SUCCESS
    }
}
