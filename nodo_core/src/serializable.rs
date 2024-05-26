// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::EyreResult;
use crate::Message;
use serde::{Deserialize, Serialize};

/// Serialized data
#[derive(Clone)]
pub struct SerializedValue {
    pub channel_id: RecorderChannelId,
    pub buffer: Vec<u8>,
}

/// A serialized message
pub type SerializedMessage = Message<SerializedValue>;

/// ID of a channel used for recording data
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RecorderChannelId(pub u16);

impl From<RecorderChannelId> for u16 {
    fn from(other: RecorderChannelId) -> u16 {
        other.0
    }
}

/// Methods to serialize data to bytes and deserialize bytes to data.
pub trait BinaryFormat<T> {
    /// Schema used for this message type
    fn schema(&self) -> Schema;

    /// Serialize data into bytes
    fn serialize(&self, data: &T) -> EyreResult<Vec<u8>>;

    /// Deserialize data from bytes
    fn deserialize(&self, buffer: Vec<u8>) -> EyreResult<T>;
}

/// Schema definition used to describe the data type of a serialized message
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Schema {
    /// Name of the type
    pub name: String,

    /// Encoding used to serialize the message, e.g. "protobuf"
    pub encoding: String,
}
