use crate::{Acqtime, Stamp, WithAcqtime};

/// A data value with timestamps and sequence number
#[derive(Debug, Clone)]
pub struct Message<T> {
    /// Sequence number as issued by transmitter
    pub seq: u64,

    /// Timestamps
    pub stamp: Stamp,

    /// Main payload of this message
    pub value: T,
}

impl<T> Message<T> {
    pub fn map<S, F>(self, f: F) -> Message<S>
    where
        F: FnOnce(T) -> S,
    {
        Message {
            seq: self.seq,
            stamp: self.stamp,
            value: f(self.value),
        }
    }
}

impl<T> WithAcqtime for Message<T> {
    fn acqtime(&self) -> Acqtime {
        self.stamp.acqtime
    }
}
