// Copyright 2023 by David Weikersdorfer. All rights reserved.

use nodo::{
    codelet::{ScheduleBuilder, ScheduleExecutor},
    prelude::*,
    runtime::Runtime,
};
use nodo_std::Terminator;
use std::time::Duration;

mod common;

#[derive(Clone)]
pub struct Ping(String);

const NUM_MESSAGES: usize = 85;

struct Alice {
    num_sent: usize,
}

#[derive(TxBundleDerive)]
struct AliceTx {
    ping: DoubleBufferTx<Ping>,
}

impl Codelet for Alice {
    type Config = ();
    type Rx = ();
    type Tx = AliceTx;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            (),
            AliceTx {
                ping: DoubleBufferTx::new(1),
            },
        )
    }

    fn step(&mut self, _: &Context<Self>, _: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        tx.ping.push(Ping(format!("hello_{}", self.num_sent)))?;
        self.num_sent += 1;
        SUCCESS
    }

    fn stop(&mut self, _: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        assert_eq!(self.num_sent, NUM_MESSAGES);
        SUCCESS
    }
}

struct Bob {
    num_recv: usize,
}

#[derive(RxBundleDerive)]
struct BobRx {
    ping: DoubleBufferRx<Ping>,
}

impl Codelet for Bob {
    type Config = ();
    type Rx = BobRx;
    type Tx = ();

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            BobRx {
                ping: DoubleBufferRx::new(OverflowPolicy::Reject(1), RetentionPolicy::Drop),
            },
            (),
        )
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        let ping = rx.ping.pop()?;
        assert_eq!(ping.0, format!("hello_{}", self.num_recv));
        self.num_recv += 1;
        SUCCESS
    }

    fn stop(&mut self, _: &Context<Self>, _: &mut Self::Rx, _: &mut Self::Tx) -> Outcome {
        assert_eq!(self.num_recv, NUM_MESSAGES);
        SUCCESS
    }
}

use std::sync::Once;

static INIT: Once = Once::new();

fn init_reporting() {
    INIT.call_once(|| {
        color_eyre::install().unwrap();
        env_logger::init();
    });
}

fn test_schedule(schedule: ScheduleExecutor) {
    let mut rt = Runtime::new();
    rt.add_codelet_schedule(schedule);
    rt.spin();
}

#[test]
fn alice_bob_codelets() {
    init_reporting();

    let mut alice = Alice { num_sent: 0 }.into_instance("alice", ());
    let mut bob = Bob { num_recv: 0 }.into_instance("bob", ());

    alice.tx.ping.connect(&mut bob.rx.ping).unwrap();

    test_schedule(
        ScheduleBuilder::new()
            .with_period(Duration::from_millis(2))
            .with_max_step_count(NUM_MESSAGES)
            .with(alice)
            .with(bob)
            .finalize(),
    );
}

#[test]
fn alice_bob_codelets_with_terminator() {
    init_reporting();

    let mut rt = Runtime::new();

    let term = Terminator::new(NUM_MESSAGES - 1, rt.tx_control()).into_instance("terminator", ());
    let mut alice = Alice { num_sent: 0 }.into_instance("alice", ());
    let mut bob = Bob { num_recv: 0 }.into_instance("bob", ());

    alice.tx.ping.connect(&mut bob.rx.ping).unwrap();

    rt.add_codelet_schedule(
        ScheduleBuilder::new()
            .with_period(Duration::from_millis(2))
            .with(term)
            .with(alice)
            .with(bob)
            .into(),
    );

    rt.spin();
}

#[test]
fn alice_double_bob_codelets() {
    init_reporting();

    let mut alice = Alice { num_sent: 0 }.into_instance("alice", ());
    let mut bob_1 = Bob { num_recv: 0 }.into_instance("bob 1", ());
    let mut bob_2 = Bob { num_recv: 0 }.into_instance("bob 2", ());

    alice.tx.ping.connect(&mut bob_1.rx.ping).unwrap();
    alice.tx.ping.connect(&mut bob_2.rx.ping).unwrap();

    test_schedule(
        ScheduleBuilder::new()
            .with_period(Duration::from_millis(2))
            .with_max_step_count(NUM_MESSAGES)
            .with(alice)
            .with(bob_1)
            .with(bob_2)
            .finalize(),
    );
}

#[test]
fn alice_many_bobs_codelets() {
    init_reporting();

    let mut alice = Alice { num_sent: 0 }.into_instance("alice", ());
    let mut bobs = (0..50)
        .map(|i| Bob { num_recv: 0 }.into_instance(format!("bob {i}"), ()))
        .collect::<Vec<_>>();

    for bob in bobs.iter_mut() {
        alice.tx.ping.connect(&mut bob.rx.ping).unwrap();
    }

    let mut schedule = ScheduleBuilder::new()
        .with_period(Duration::from_millis(2))
        .with_max_step_count(NUM_MESSAGES)
        .with(alice);

    for bob in bobs.into_iter() {
        schedule.append(bob);
    }

    test_schedule(schedule.finalize());
}
