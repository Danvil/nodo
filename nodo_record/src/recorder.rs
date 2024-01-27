// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::{McapWriter, McapWriterConfig, Serializer};
use mcap::{Channel as McapChannel, Schema as McapSchema};
use nodo::channels::DoubleBufferTx;
use nodo::codelet::{
    CodeletInstance, Instantiate, IntoInstance, Schedulable, ScheduleBuilder, Vise,
};
use nodo_core::SchemaDb;
use nodo_core::{
    eyre, EyreResult, ProtoSerializable, RecorderChannelId, SerializedMessage, WithAcqtime,
};
use nodo_std::Join;
use nodo_std::JoinConfig;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Faciliates recording of data channels
pub struct Recorder {
    rec: CodeletInstance<McapWriter<'static>>,
    join: CodeletInstance<Join<SerializedMessage>>,
    ser_vises: Vec<Vise>,
}

impl Recorder {
    /// Create a new recorder which writes to an MCAP file
    pub fn new(cfg: McapWriterConfig) -> EyreResult<Self> {
        let mut join = Join::instantiate("rec-join", JoinConfig { input_count: 0 });
        let mut rec = McapWriter::from_config(&cfg)?.into_instance("rec-writer", cfg);

        join.tx.output.connect(&mut rec.rx.0)?;

        Ok(Self {
            join,
            rec,
            ser_vises: Vec::new(),
        })
    }

    pub fn schema_db_mut(&mut self) -> &mut SchemaDb {
        &mut self.rec.state.schema_db
    }

    /// Creates a codelet which serializes messages to protobuf and writes them to MCAP.
    #[must_use]
    pub fn record<S, T>(&mut self, topic: S, tx: &mut DoubleBufferTx<T>) -> EyreResult<()>
    where
        S: Into<String>,
        T: Send + Sync + Clone + WithAcqtime + ProtoSerializable + 'static,
    {
        let topic = topic.into();
        let codelet_name = format!("rec-{}", topic);

        let schema = T::schema();

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

        let mut ser = Serializer::new(channel_id).into_instance(codelet_name, ());

        tx.connect(&mut ser.rx.0)?;
        ser.tx.0.connect(&mut self.join.rx.new_channel_mut())?;

        self.ser_vises.push(ser.into());

        Ok(())
    }
}

impl Schedulable for Recorder {
    fn schedule(self, sched: &mut ScheduleBuilder) {
        self.ser_vises.schedule(sched);
        self.join.schedule(sched);
        self.rec.schedule(sched);
    }
}
