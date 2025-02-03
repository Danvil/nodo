// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::{EyreResult, Message};

/// A message with a topic. Used by certain codelets to identify messages.
#[derive(Clone)]
pub struct WithTopic<T> {
    /// The topic of the message
    pub topic: Topic,

    /// The actual message
    pub value: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topic {
    Text(String),
    Id(u64),
}

impl<'a> From<&'a str> for Topic {
    fn from(text: &'a str) -> Self {
        Topic::Text(String::from(text))
    }
}

impl From<&Topic> for String {
    fn from(topic: &Topic) -> Self {
        match topic {
            Topic::Text(text) => text.clone(),
            Topic::Id(id) => id.to_string(),
        }
    }
}

/// A serialized message
pub type SerializedMessage = Message<Vec<u8>>;

/// Methods to serialize data to bytes and deserialize bytes to data.
pub trait BinaryFormat<T> {
    /// Schema used for this message type
    fn schema(&self) -> Schema;

    /// Serialize data into bytes
    fn serialize(&mut self, data: &T) -> EyreResult<Vec<u8>>;

    /// Deserialize data from bytes
    fn deserialize(&mut self, buffer: &[u8]) -> EyreResult<T>;
}

/// Schema definition used to describe the data type of a serialized message
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Schema {
    /// Name of the type
    pub name: String,

    /// Encoding used to serialize the message, e.g. "protobuf"
    pub encoding: String,
}
