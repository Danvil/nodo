// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::EyreResult;
use crate::NngPubSubHeader;
use log::error;
use log::info;
use log::trace;
use nng::options::protocol::pubsub::Subscribe;
use nng::options::Options;
use nng::Protocol;
use nng::Socket;
use nodo::prelude::*;
use nodo_core::eyre;
use nodo_core::Topic;
use nodo_core::WithTopic;

/// Codelet which receives serialized messages and writes them to MCAP
pub struct NngSub {
    socket: Option<Socket>,
    message_count: usize,
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
        }
    }
}

impl Codelet for NngSub {
    type Config = NngSubConfig;
    type Rx = ();
    type Tx = DoubleBufferTx<Message<WithTopic<Vec<u8>>>>;

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

        let mut received_count = 0;

        loop {
            match socket.try_recv() {
                Ok(buff) => match Self::parse(buff) {
                    Ok(msg) => {
                        tx.push(msg)?;
                        self.message_count += 1;
                        received_count += 1;
                    }
                    Err(err) => {
                        log::error!("{err:?}");
                    }
                },
                Err(nng::Error::TryAgain) => {
                    break;
                }
                Err(err) => Err(err)?,
            }
        }

        if received_count > 0 {
            SUCCESS
        } else {
            SKIPPED
        }
    }
}

impl NngSub {
    fn parse(msg: nng::Message) -> EyreResult<Message<WithTopic<Vec<u8>>>> {
        // Message has three parts:
        let data = msg.as_slice();

        // 1) topic: null-terminated string
        let (cstr, data) = parse_cstr(data)?;
        let topic: Topic = cstr.into();

        // 2) header: NngPubSubHeader
        let header: NngPubSubHeader =
            bincode::deserialize(&data[0..NngPubSubHeader::BINCODE_SIZE])?;
        if header.magic != NngPubSubHeader::MAGIC {
            return Err(eyre!("invalid header magic"));
        }

        // 3) value: [u8]
        let value = data[NngPubSubHeader::BINCODE_SIZE..].to_vec();
        let checksum = NngPubSubHeader::CRC.checksum(&value);
        if header.payload_checksum != checksum {
            return Err(eyre!(
                "payload failed checksum test: expected={}, actual={}",
                header.payload_checksum,
                checksum
            ));
        }

        Ok(Message {
            seq: header.seq,
            stamp: header.stamp,
            value: WithTopic { topic, value },
        })
    }
}

fn parse_cstr(utf8_src: &[u8]) -> EyreResult<(&str, &[u8])> {
    let end = utf8_src
        .iter()
        .position(|&c| c == b'\0')
        .ok_or_else(|| eyre!("null terminator not found"))?;

    Ok(::std::str::from_utf8(&utf8_src[0..end]).map(|x| (x, &utf8_src[end + 1..]))?)
}
