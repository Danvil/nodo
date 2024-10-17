// Copyright 2023 by David Weikersdorfer. All rights reserved.

use nodo::{prelude::*, runtime::RuntimeControl};

/// Terminates after certain number of steps.
pub struct Terminator {
    countdown: usize,
    tx_control: std::sync::mpsc::SyncSender<RuntimeControl>,
}

impl Terminator {
    pub fn new(countdown: usize, tx_control: std::sync::mpsc::SyncSender<RuntimeControl>) -> Self {
        Self {
            countdown,
            tx_control,
        }
    }
}

impl Codelet for Terminator {
    type Status = DefaultStatus;
    type Config = ();
    type Rx = ();
    type Tx = ();

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        ((), ())
    }

    fn step(&mut self, _: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        if self.countdown == 0 {
            self.tx_control.send(RuntimeControl::RequestStop)?;
            SUCCESS
        } else {
            self.countdown -= 1;
            SUCCESS
        }
    }
}
