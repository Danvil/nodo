// Copyright 2023 by David Weikersdorfer. All rights reserved.

mod clock;
#[macro_use]
mod outcome;
mod serializable;
mod stamped;
mod timestamp;

pub use clock::*;
pub use outcome::*;
pub use serializable::*;
pub use stamped::*;
pub use timestamp::*;

pub trait WithAcqtime {
    fn acqtime(&self) -> &Timestamp;
}
