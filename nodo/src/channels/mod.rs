// Copyright 2023 by David Weikersdorfer. All rights reserved.
use core::fmt;

mod bundle;
mod double_buffer_channel;
mod stage_queue;
mod timeseries;

pub use bundle::*;
pub use double_buffer_channel::*;
pub use stage_queue::*;
pub use timeseries::*;

/// Statistics about a channel sync operation
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncResult {
    /// Number of messages which where moved into the channel
    pub received: usize,

    /// Number of messges which were forgotten by the receiver to store incoming messages
    pub forgotten: usize,

    /// Number of messages which where dropped by the receiver
    pub dropped: usize,

    /// Retention policy "EnforceEmpty" in use but the receiver queue was not empty.
    pub enforce_empty_violation: bool,
}

impl SyncResult {
    pub const ZERO: SyncResult = SyncResult {
        received: 0,
        forgotten: 0,
        dropped: 0,
        enforce_empty_violation: false,
    };
}

/// Result of a channel flush operation. This type combines statistics and potential errors.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FlushResult {
    /// Number of (unique) messages which where available for publish.
    pub available: usize,

    /// Number of messages which where cloned. If there is more than one connection messages
    /// published to additional receivers are clones.
    pub cloned: usize,

    /// Total number of messages successfully transmitted to all connections.
    pub published: usize,

    /// Stores error indicators for each connection. Flush can fail to transmitt a message to the
    /// RX in certain conditions, for example if the receiving channel is full while using a
    /// reject policy.
    pub error_indicator: FlushErrorIndicator,
}

impl FlushResult {
    pub const ZERO: FlushResult = FlushResult {
        available: 0,
        published: 0,
        cloned: 0,
        error_indicator: FlushErrorIndicator::NO_ERROR,
    };
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FlushErrorIndicator {
    marks: u64,
}

impl FlushErrorIndicator {
    pub const NO_ERROR: FlushErrorIndicator = FlushErrorIndicator { marks: 0 };

    pub fn new() -> Self {
        Self { marks: 0 }
    }

    pub fn mark(&mut self, i: usize) {
        self.marks &= 1 << i;
    }

    pub fn is_err(&self) -> bool {
        self.marks != 0
    }

    pub fn get(&self, i: usize) -> bool {
        (self.marks & (1 << i)) != 0
    }
}

impl fmt::Display for FlushErrorIndicator {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "FlushErrorIndicator({:b})", self.marks)
    }
}
