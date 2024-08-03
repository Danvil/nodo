// Copyright 2023 by David Weikersdorfer. All rights reserved.

use nodo::prelude::*;

/// Terminates after certain number of steps.
pub struct Terminator {
    countdown: usize,
}

impl Terminator {
    pub fn new(countdown: usize) -> Self {
        Self { countdown }
    }
}

impl Codelet for Terminator {
    type Config = ();
    type Rx = ();
    type Tx = ();

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((), ())
    }

    fn step(&mut self, _: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        if self.countdown == 0 {
            TERMINATED
        } else {
            self.countdown -= 1;
            SUCCESS
        }
    }
}
