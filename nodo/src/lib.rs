// Copyright 2023 by David Weikersdorfer. All rights reserved.

pub mod channels;
pub mod codelet;
pub mod runtime;
pub mod sleep;
pub mod task;
pub mod zield;

pub mod prelude {
    pub use crate::{
        channels::{
            connect, Connect, DoubleBufferRx, DoubleBufferTx, OverflowPolicy, Pop, RetentionPolicy,
            Rx, Timeseries, Tx,
        },
        codelet::{Codelet, Context, Instantiate, IntoInstance, Sequence},
    };
    pub use nodo_core::{
        Acqtime, Clock, Message, Outcome, OutcomeKind, Pubtime, Stamp, WithAcqtime, RUNNING,
        SKIPPED, SUCCESS, TERMINATED,
    };
    pub use nodo_derive::{RxBundleDerive, TxBundleDerive};
}
