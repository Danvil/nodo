// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::NngPubSubHeader;
use log::{error, info, trace};
use nng::{Protocol, Socket};
use nodo::prelude::*;
use nodo_core::{Topic, WithTopic};

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
    type Rx = DoubleBufferRx<Message<WithTopic<Vec<u8>>>>;
    type Tx = ();

    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new(
                OverflowPolicy::Forget(cfg.queue_size),
                RetentionPolicy::Drop,
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

    fn stop(&mut self, _: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        // SAFETY: guaranteed by start
        let socket = self.socket.take().unwrap();

        socket.close();

        SUCCESS
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        // SAFETY: guaranteed by start
        let socket = self.socket.as_mut().unwrap();

        let mut count = 0;
        while let Some(message) = rx.try_pop() {
            let topic_buffer = serialize_topic(&message.value.topic);

            let header = NngPubSubHeader {
                magic: NngPubSubHeader::MAGIC,
                seq: message.seq.try_into()?,
                stamp: message.stamp,
                payload_checksum: NngPubSubHeader::CRC.checksum(&message.value.value),
            };
            let header_buffer = bincode::serialize(&header)?;

            let outmsg_size = topic_buffer.len() + header_buffer.len() + message.value.value.len();
            let mut outmsg = nng::Message::with_capacity(outmsg_size);
            outmsg.push_back(&topic_buffer);
            outmsg.push_back(&header_buffer);
            outmsg.push_back(&message.value.value);

            socket.send(outmsg).map_err(|(_, err)| err)?;

            count += 1;
        }

        self.message_count += count;

        if count > 0 {
            SUCCESS
        } else {
            SKIPPED
        }
    }
}

fn serialize_topic(topic: &Topic) -> Vec<u8> {
    let mut out = match topic {
        Topic::Text(text) => text.as_bytes().to_vec(),
        Topic::Id(id) => id.to_string().as_bytes().to_vec(),
    };

    // The string itself must not contain any NULL terminators.
    assert!(out.iter().all(|&b| b != 0));

    // The serialized string must be NULL terminated for NNG to work as a topic.
    out.push(0);

    out
}
