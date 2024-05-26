// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::NngPubSubHeader;
use crate::SerializedValue;
use log::error;
use log::info;
use log::trace;
use nng::options::protocol::pubsub::Subscribe;
use nng::options::Options;
use nng::Protocol;
use nng::Socket;
use nodo::prelude::*;
use nodo_core::SerializedMessage;

/// Codelet which receives serialized messages and writes them to MCAP
pub struct NngSub {
    socket: Option<Socket>,
    message_count: usize,
    parser: MessageParser,
}

pub struct NngSubConfig {
    pub address: String,
    pub queue_size: usize,
}

impl Default for NngSub {
    fn default() -> Self {
        Self {
            socket: None,
            message_count: 0,
            parser: MessageParser::default(),
        }
    }
}

impl Codelet for NngSub {
    type Config = NngSubConfig;
    type Rx = ();
    type Tx = DoubleBufferTx<SerializedMessage>;

    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((), DoubleBufferTx::new(cfg.queue_size))
    }

    fn start(&mut self, cx: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        info!("Opening SUB socket at '{}'..", cx.config.address);

        let socket = Socket::new(Protocol::Sub0)?;

        socket.pipe_notify(move |_, ev| {
            trace!("nng::socket::pipe_notify: {ev:?}");
        })?;

        let res = socket.dial_async(&cx.config.address);

        // subscribe to all topics
        socket.set_opt::<Subscribe>(vec![])?;

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

    fn step(&mut self, _cx: &Context<Self>, _rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        // SAFETY: guaranteed by start
        let socket = self.socket.as_mut().unwrap();

        loop {
            match socket.try_recv() {
                Ok(buff) => {
                    if let Some(msg) = self.parser.push(buff.to_vec()) {
                        tx.push(msg)?;
                        self.message_count += 1;
                    }
                }
                Err(nng::Error::TryAgain) => {
                    break;
                }
                Err(err) => Err(err)?,
            }
        }

        SUCCESS
    }
}

/// Helps us figure out which packets are header and which are payload. Also needs to be robust
/// against dropped messages and messages received out of order.
#[derive(Default)]
struct MessageParser {
    head: Option<Vec<u8>>,
}

impl MessageParser {
    pub fn push(&mut self, tail: Vec<u8>) -> Option<SerializedMessage> {
        if let Some(head) = self.head.take() {
            // verify that (head, tail) form a message
            if let Ok::<NngPubSubHeader, _>(header) = bincode::deserialize(&head) {
                let checksum = NngPubSubHeader::CRC.checksum(&tail);
                if header.magic == NngPubSubHeader::MAGIC && header.payload_checksum == checksum {
                    Some(SerializedMessage {
                        seq: header.seq,
                        stamp: header.stamp,
                        value: SerializedValue {
                            channel_id: header.channel_id,
                            buffer: tail,
                        },
                    })
                } else {
                    // something is wrong ...  skip one packet
                    self.head = Some(tail);
                    None
                }
            } else {
                // something is wrong ...  skip one packet
                self.head = Some(tail);
                None
            }
        } else {
            // first packet
            self.head = Some(tail);
            None
        }
    }
}
