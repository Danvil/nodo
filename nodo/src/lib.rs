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
        codelet::{
            Codelet, CodeletStatus, Context, Instantiate, IntoInstance, Schedulable, Sequence,
            Sequenceable,
        },
    };
    pub use nodo_core::{
        Acqtime, Clock, DefaultStatus, Message, Outcome, OutcomeKind, Pubtime, Stamp, WithAcqtime,
        RUNNING, SKIPPED, SUCCESS,
    };
    pub use nodo_derive::{RxBundleDerive, Status, TxBundleDerive};
}
