// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::EyreResult;
use crate::Message;
use std::collections::HashMap;

/// Serialized data
#[derive(Clone)]
pub struct SerializedValue {
    pub channel_id: RecorderChannelId,
    pub buffer: Vec<u8>,
}

/// A serialized message
pub type SerializedMessage = Message<SerializedValue>;

/// ID of a channel used for recording data
#[derive(Clone, Copy)]
pub struct RecorderChannelId(pub u16);

impl From<RecorderChannelId> for u16 {
    fn from(other: RecorderChannelId) -> u16 {
        other.0
    }
}

/// Types which can be serialize using protobuf
pub trait ProtoSerializable {
    /// Schema used for this message type
    fn schema() -> Schema;

    /// Serializes itself into bytes
    fn into_proto(self) -> EyreResult<Vec<u8>>;
}

/// Schema definition used to describe the data type of a serialized message
#[derive(Eq, Hash, PartialEq, Debug)]
pub struct Schema {
    /// Name of the type
    pub name: String,

    /// Encoding used to serialize the message, e.g. "protobuf"
    pub encoding: String,
}

/// Collection of known schemas
///
/// See also: https://mcap.dev/spec/registry#well-known-schema-encodings
#[derive(Default)]
pub struct SchemaDb {
    schemas: HashMap<Schema, &'static [u8]>,
}

impl SchemaDb {
    pub fn insert(&mut self, schema: Schema, def: &'static [u8]) {
        self.schemas.insert(schema, def);
    }

    /// Looks up a schema
    pub fn lookup(&self, schema: &Schema) -> Option<&'static [u8]> {
        self.schemas.get(schema).copied()
    }
}
