// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::SchemaSet;
use crate::{McapWriter, McapWriterConfig};
use mcap::{Channel as McapChannel, Schema as McapSchema};
use nodo::codelet::{CodeletInstance, Schedulable, ScheduleBuilder, Vise};
use nodo::prelude::*;
use nodo_core::BinaryFormat;
use nodo_core::{eyre, EyreResult, RecorderChannelId, SerializedMessage};
use nodo_std::Join;
use nodo_std::JoinConfig;
use nodo_std::Serializer;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Faciliates recording of data channels
pub struct Recorder<BF> {
    serializer: BF,
    rec: CodeletInstance<McapWriter<'static>>,
    join: CodeletInstance<Join<SerializedMessage>>,
    ser_vises: Vec<Vise>,
}

impl<BF> Recorder<BF> {
    /// Create a new recorder which writes to an MCAP file
    pub fn new(serializer: BF, cfg: McapWriterConfig) -> EyreResult<Self> {
        let mut join = Join::instantiate("rec-join", JoinConfig { input_count: 0 });
        let mut rec = McapWriter::from_config(&cfg)?.into_instance("rec-writer", cfg);

        join.tx.output.connect(&mut rec.rx.0)?;

        Ok(Self {
            serializer,
            join,
            rec,
            ser_vises: Vec::new(),
        })
    }

    pub fn schema_db_mut(&mut self) -> &mut SchemaSet {
        &mut self.rec.state.schema_db
    }

    /// Creates a codelet which serializes messages to protobuf and writes them to MCAP.
    #[must_use]
    pub fn record<S, T>(&mut self, topic: S, tx: &mut DoubleBufferTx<Message<T>>) -> EyreResult<()>
    where
        BF: Clone + Send + BinaryFormat<T> + 'static,
        S: Into<String>,
        T: Clone + Send + Sync + 'static,
    {
        let topic = topic.into();
        let codelet_name = format!("rec-{}", topic);

        let schema = self.serializer.schema();

        let schema_def = self
            .rec
            .state
            .schema_db
            .lookup(&schema)
            .ok_or_else(|| eyre!("unknown schema: {schema:?}"))?;

        let mcap_schema = McapSchema {
            name: schema.name,
            encoding: schema.encoding.clone(),
            data: Cow::from(&schema_def[..]),
        };

        self.rec.state.channels.push(McapChannel {
            topic,
            schema: Some(Arc::new(mcap_schema)),
            message_encoding: schema.encoding,
            metadata: BTreeMap::default(),
        });

        let channel_id = RecorderChannelId(
            self.rec
                .state
                .writer
                .add_channel(&self.rec.state.channels.last().unwrap())?,
        );

        let mut ser =
            Serializer::new(channel_id, self.serializer.clone()).into_instance(codelet_name, ());

        tx.connect(&mut ser.rx)?;
        ser.tx.connect(&mut self.join.rx.new_channel_mut())?;

        self.ser_vises.push(ser.into());

        Ok(())
    }
}

impl<BF> Schedulable for Recorder<BF> {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        self.ser_vises.schedule(sched);
        self.join.schedule(sched);
        self.rec.schedule(sched);
    }
}
