// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::SchemaSet;
use log::{error, trace};
use mcap::{
    records::MessageHeader as McapMessageHeader, Channel as McapChannel,
    WriteOptions as McapWriterOptions, Writer as McapWriterImpl,
};
use nodo::channels::DoubleBufferRx;
use nodo::channels::Pop;
use nodo::codelet::Codelet;
use nodo::codelet::Context;
use nodo_core::{Outcome, SerializedMessage};

use nodo_core::{eyre, EyreResult, WrapErr, SUCCESS};

/// Codelet which receives serialized messages and writes them to MCAP
pub struct McapWriter<'a> {
    pub(crate) schema_db: SchemaSet,
    pub(crate) channels: Vec<McapChannel<'a>>,
    pub(crate) writer: McapWriterImpl<'a, std::io::BufWriter<std::fs::File>>,
    message_count: usize,
    unflushed_message_count: usize,
}

pub struct McapWriterConfig {
    pub path: String,
    pub enable_compression: bool,
    pub chunk_message_count: usize,
}

impl McapWriter<'_> {
    pub fn from_config(cfg: &McapWriterConfig) -> EyreResult<Self> {
        assert!(
            cfg.chunk_message_count > 0,
            "chunk_message_count must be at least 1"
        );

        let file = std::fs::File::create(&cfg.path)
            .wrap_err_with(|| eyre!("could not create file '{}'", cfg.path))?;

        let writer = McapWriterOptions::new()
            .compression(if cfg.enable_compression {
                Some(mcap::Compression::Lz4)
            } else {
                None
            })
            .chunk_size(None) // we flush manually by message count
            .create(std::io::BufWriter::new(file))
            .wrap_err_with(|| eyre!("could not create MCAP writer for file '{}", cfg.path))?;

        let schema_db = SchemaSet::default();

        Ok(Self {
            writer,
            channels: Vec::new(),
            schema_db,
            message_count: 0,
            unflushed_message_count: 0,
        })
    }
}

impl Codelet for McapWriter<'_> {
    type Config = McapWriterConfig;
    type Rx = (DoubleBufferRx<SerializedMessage>,);
    type Tx = ();

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((DoubleBufferRx::new_auto_size(),), ())
    }

    fn start(&mut self, _cx: &Context<Self>, _rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        assert!(
            self.message_count == 0,
            "McapWriter restart not implemented",
        );
        SUCCESS
    }

    fn step(&mut self, cx: &Context<Self>, rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        // TODO implement policies to drop messages when queue gets too full

        let mut count = 0;
        while let Some(message) = rx.0.try_pop() {
            match self.write_message(message) {
                Ok(()) => count += 1,
                Err(err) => error!("error writing message to MCAP file: {err:?}"),
            }
        }

        self.message_count += count;
        self.unflushed_message_count += count;

        if self.unflushed_message_count >= cx.config.chunk_message_count {
            trace!(
                "flushed chunk with {} messages",
                self.unflushed_message_count
            );

            self.writer.flush()?;
            self.unflushed_message_count = 0;
        }

        SUCCESS
    }

    fn stop(&mut self, _cx: &Context<Self>, _rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        trace!(
            "finished last chunk with {} messages",
            self.unflushed_message_count
        );

        self.writer.finish()?;

        SUCCESS
    }
}

impl McapWriter<'_> {
    fn write_message(&mut self, message: SerializedMessage) -> EyreResult<()> {
        self.writer.write_to_known_channel(
            &McapMessageHeader {
                channel_id: message.value.channel_id.into(),
                sequence: message.seq.try_into().unwrap(),
                log_time: message.stamp.acqtime.as_nanos().try_into()?,
                publish_time: message.stamp.pubtime.as_nanos().try_into()?,
            },
            &message.value.buffer,
        )?;
        Ok(())
    }
}
