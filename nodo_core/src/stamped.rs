// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::Timestamp;
use core::ops::Deref;

/// A payload with timestamp
#[derive(Debug, Clone)]
pub struct Stamped<T> {
    pub acqtime: Timestamp,
    pub pubtime: Timestamp,
    pub data: T,
}

impl<T> Deref for Stamped<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}
