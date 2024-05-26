// Copyright 2023 by David Weikersdorfer. All rights reserved.

mod bundle;
mod double_buffer_channel;
mod stage_queue;

pub use bundle::*;
pub use double_buffer_channel::*;
pub use stage_queue::*;

// use nodo_core::Acqtime;
// use nodo_core::Message;

// pub type Tx<T> = DoubleBufferTx<Message<T>>;

// pub type Rx<T> = DoubleBufferRx<Message<T>>;

// impl<T: Send + Sync> Tx<Message<T>> {
//     pub fn push(&mut self, acqtime: Acqtime, value: T) -> Result<(), TxSendError> {
//         todo!()
//     }
// }
