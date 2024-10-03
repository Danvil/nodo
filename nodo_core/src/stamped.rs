// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::Timestamp;
use core::{fmt, ops, time::Duration};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Stamp {
    /// Time at which data was acquired by the hardware
    pub acqtime: Acqtime,

    /// Time at which the message was published by the transmitter
    pub pubtime: Pubtime,
}

impl fmt::Debug for Stamp {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            fmt,
            "Stamp {{ acq: {:?}, pub: {:?} }}",
            *self.acqtime, *self.pubtime
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AcqtimeMarker;

pub type Acqtime = Timestamp<AcqtimeMarker>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
