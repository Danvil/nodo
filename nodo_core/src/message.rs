use crate::Stamp;

/// A data value with timestamps and sequence number
#[derive(Clone)]
pub struct Message<T> {
    /// Sequence number as issued by transmitter
    pub seq: usize,

    /// Timestamps
    pub stamp: Stamp,

    /// Main payload of this message
    pub value: T,
}
