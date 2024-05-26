// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::Timestamp;
use core::ops;
use core::time::Duration;

#[derive(Clone)]
pub struct Stamp {
    /// Time at which data was acquired by the hardware
    pub acqtime: Acqtime,

    /// Time at which the message was published by the transmitter
    pub pubtime: Pubtime,
}

#[derive(Debug, Clone, Copy)]
pub struct AcqtimeMarker;

pub type Acqtime = Timestamp<AcqtimeMarker>;

#[derive(Debug, Clone, Copy)]
pub struct PubtimeMarker;

pub type Pubtime = Timestamp<PubtimeMarker>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampKind {
    Acq,
    Pub,
}

impl ops::Index<TimestampKind> for Stamp {
    type Output = Duration;

    fn index(&self, idx: TimestampKind) -> &Self::Output {
        match idx {
            TimestampKind::Acq => &self.acqtime,
            TimestampKind::Pub => &self.pubtime,
        }
    }
}

pub trait WithAcqtime {
    fn acqtime(&self) -> Acqtime;
}
