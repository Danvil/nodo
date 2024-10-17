// Copyright 2024 by David Weikersdorfer. All rights reserved.

use core::time::Duration;
use eyre::Result;
use nodo::{codelet::ScheduleBuilder, prelude::*, runtime::Runtime};
use nodo_std::Terminator;

#[derive(Clone)]
pub struct Ping;

struct Pinger {
    num_sent: usize,
}

#[derive(Status)]
enum PingerStatus {
    #[default]
    #[skipped]
    Idle,

    #[label = "ping"]
    Pinging(usize),
}

#[derive(TxBundleDerive)]
struct PingerTx {
    ping: DoubleBufferTx<Ping>,
}

impl Codelet for Pinger {
    type Status = PingerStatus;
    type Config = ();
    type Rx = ();
    type Tx = PingerTx;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            (),
            PingerTx {
                ping: DoubleBufferTx::new(1),
            },
        )
    }

    fn step(
        &mut self,
        _: &Context<Self>,
        _: &mut Self::Rx,
        tx: &mut Self::Tx,
    ) -> Result<PingerStatus> {
        tx.ping.push(Ping)?;
        self.num_sent += 1;
        Ok(PingerStatus::Pinging(self.num_sent))
    }
}

#[test]
fn test_custom_status() {
    let mut rt = Runtime::new();

    let term = Terminator::new(100, rt.tx_control()).into_instance("terminator", ());

    let alice = Pinger { num_sent: 0 }.into_instance("alice", ());

    rt.add_codelet_schedule(
        ScheduleBuilder::new()
            .with_period(Duration::from_millis(2))
            .with(term)
            .with(alice)
            .into(),
    );

    rt.spin();
}
