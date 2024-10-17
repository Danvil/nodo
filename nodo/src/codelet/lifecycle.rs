// Copyright 2024 by David Weikersdorfer. All rights reserved.

use crate::codelet::Transition;
use eyre::Result;
use nodo_core::DefaultStatus;

pub trait Lifecycle {
    /// Applies a lifecycel change
    fn cycle(&mut self, transition: Transition) -> Result<DefaultStatus>;
}
