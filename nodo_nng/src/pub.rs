// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::NngPubSubHeader;
use log::{error, info, trace};
use nng::{Protocol, Socket};
use nodo::prelude::*;
use nodo_core::{Topic, WithTopic};
use std::{collections::HashMap, time::Instant};

/// Codelet which receives serialized messages and writes them to MCAP
pub struct NngPub {
    socket: Option<Socket>,
    statistics: Option<Statistics>,
}

pub struct NngPubConfig {
    pub address: String,
    pub queue_size: usize,
    pub enable_statistics: bool,
}

#[derive(Default)]
pub struct Statistics {
    items: HashMap<String, TopicStatistics>,
    last_sec: Option<Instant>,
}

impl Statistics {
    pub fn add(&mut self, topic: &str, size: usize) {
        if let Some(item) = self.items.get_mut(topic) {
            item.add(size);
        } else {
            let mut item = TopicStatistics::default();
            item.add(size);
            self.items.insert(topic.into(), item);
        }
    }

    pub fn step(&mut self) {
        let now = Instant::now();
        if self.last_sec.is_none() {
            self.last_sec = Some(now);
        }
        let delta = (now - self.last_sec.unwrap()).as_secs_f64();
        if delta >= 1.0 {
            for (_, item) in self.items.iter_mut() {
                item.reset_sec();
            }
            self.last_sec = Some(now);
            self.print_report();
        }
    }

    pub fn print_report(&self) {
        println!("NngPub statistics:");
        for (topic, item) in self.items.iter() {
            println!(
                "  [{topic}] {} Hz | {:.1} kB/s",
                item.last_sec_count,
                item.last_sec_size as f64 / 1024.
            );
        }
    }
}

#[derive(Default)]
pub struct TopicStatistics {
    total_size: usize,
    total_count: usize,
    current_sec_size: usize,
    current_sec_count: usize,
    last_sec_size: usize,
    last_sec_count: usize,
}

impl TopicStatistics {
    pub fn add(&mut self, size: usize) {
        self.total_size += size;
        self.total_count += 1;
        self.current_sec_size += size;
        self.current_sec_count += 1;
    }

    pub fn reset_sec(&mut self) {
        self.last_sec_size = self.current_sec_size;
        self.last_sec_count = self.current_sec_count;
        self.current_sec_size = 0;
        self.current_sec_count = 0;
    }
}

impl Default for NngPub {
    fn default() -> Self {
        Self {
            socket: None,
            statistics: None,
        }
    }
}

impl Codelet for NngPub {
    type Status = DefaultStatus;
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

        if cx.config.enable_statistics {
            self.statistics = Some(Statistics::default());
        }

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

            if let Some(stats) = self.statistics.as_mut() {
                let topic_str: String = (&message.value.topic).into();
                stats.add(&topic_str, outmsg_size);
            }
        }

        if let Some(stats) = self.statistics.as_mut() {
            stats.step();
        }

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
