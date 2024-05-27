// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::channels::BackStage;
use crate::channels::ConnectionCheck;
use crate::channels::FrontStage;
use crate::channels::MultiFlushError;
use crate::channels::OverflowPolicy;
use crate::channels::RxBundle;
use crate::channels::RxChannelTimeseries;
use crate::channels::StrictlyIncreasingLinear;
use crate::channels::TxBundle;
use crate::channels::{Rx, Tx};
use crate::prelude::RetentionPolicy;
use core::num::NonZeroU64;
use core::ops;
use nodo_core::Message;
use nodo_core::TimestampKind;
use std::collections::vec_deque;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;

/// Creates a new double-buffered SP-MC (single producer, multiple consumer) with fixed capacity.
pub fn fixed_channel<T: Clone + Send + Sync>(
    size: usize,
) -> (DoubleBufferTx<T>, DoubleBufferRx<T>) {
    let mut tx = DoubleBufferTx::new(size);
    let mut rx = DoubleBufferRx::new(OverflowPolicy::Reject(size), RetentionPolicy::Keep);
    tx.connect(&mut rx).unwrap();
    (tx, rx)
}

/// The maximum number of receivers which can be connected to a single transmitter. This is a
/// technical limitation as some error codes use 64-bit bitmasks.
pub const MAX_RECEIVER_COUNT: usize = 64;

/// The transmitting side of a double-buffered SP-MC channel
///
/// Messages in the outbox are sent to all connected receivers. Each receiver gets its own copy.
/// If there is more than one receiver `clone` is used to duplicate the message. Messages with
/// large data blocks should use memory sharing like `Rc` to avoid costly memory copies.
pub struct DoubleBufferTx<T> {
    outbox: BackStage<T>,
    connections: Vec<SharedBackStage<T>>,
}

/// The receiving side of a double-buffered SP-MC channel
///
/// A FIFO queue using two buffers: a front stage and a back stage. A transmitter is adding items
/// to the back stage when the transmitter is flushed. Items are moved to the front stage when
/// with sync.
///
/// Note that `sync` will clear all remaining items from the front
/// stage and move all items from the back stage to the front stage. Thus queue overflow can only
/// happen during `push`.
pub struct DoubleBufferRx<T> {
    back: SharedBackStage<T>,
    front: FrontStage<T>,
    is_connected: bool,
}

type SharedBackStage<T> = Arc<RwLock<BackStage<T>>>;

impl<T> DoubleBufferTx<T> {
    /// Creates a new TX channel with fixed capacity
    /// TODO rename to `new_fixed`
    pub fn new(capacity: usize) -> Self {
        Self {
            outbox: BackStage::new(OverflowPolicy::Reject(capacity), RetentionPolicy::Drop),
            connections: Vec::new(),
        }
    }

    /// Creates a TX channel which automatically resizes itself to always succeed in sending
    /// all messages.
    /// WARNING: This might lead to data congestion and infinitely growing queues. Usually it is
    /// better to use a fixed capacity or to forget old messages.
    pub fn new_auto_size() -> Self {
        Self {
            outbox: BackStage::new(
                OverflowPolicy::Resize(StrictlyIncreasingLinear::from_factor(2)),
                RetentionPolicy::Drop,
            ),
            connections: Vec::new(),
        }
    }

    /// Puts a message in the outbox
    pub fn push(&mut self, value: T) -> Result<(), TxSendError> {
        self.outbox.push(value).map_err(|_| TxSendError::QueueFull)
    }

    /// Puts multiple messages in the outbox
    pub fn push_many<I: IntoIterator<Item = T>>(&mut self, values: I) -> Result<(), TxSendError> {
        for x in values.into_iter() {
            self.push(x)?;
        }
        Ok(())
    }

    /// Connects a receiver to this transmitter
    ///
    /// Receivers must be connected to at most one transmitter. There is also a technical connection
    /// limit per transmitter (64 at the moment). Certain policy combinations are forbidden. For
    /// example it is an error to connect a receiver with the "Reject" policy to a transmitter
    /// with the "Resize" policy as this will lead to failed message passing.
    pub fn connect(&mut self, rx: &mut DoubleBufferRx<T>) -> Result<(), TxConnectError>
    where
        T: Send + Sync,
    {
        if rx.is_connected() {
            return Err(TxConnectError::ReceiverAlreadyConnected);
        }

        if self.connections.len() >= MAX_RECEIVER_COUNT {
            return Err(TxConnectError::MaxConnectionCountExceeded);
        }

        if matches!(self.outbox.overflow_policy(), OverflowPolicy::Resize(_))
            && matches!(
                rx.back.read().unwrap().overflow_policy(),
                OverflowPolicy::Reject(_)
            )
        {
            return Err(TxConnectError::PolicyMismatch);
        }

        self.connections.push(rx.back.clone());
        rx.is_connected = true;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TxConnectError {
    #[error("RX cannot be connected to more than one transmitter")]
    ReceiverAlreadyConnected,

    #[error("TX exceeded maximum connection count")]
    MaxConnectionCountExceeded,

    #[error(
        "Cannot connect a TX with policy `Resize` to an RX with policy `Reject`.
             Either change the TX policy to `Reject` or the RX policy to `Resize` or `Forget`."
    )]
    PolicyMismatch,
}

impl<T: Send + Sync + Clone> Tx for DoubleBufferTx<T> {
    fn flush(&mut self) -> Result<(), FlushError> {
        let mut result = FlushResult::new();

        // clone messages for connections 2..N
        for (i, rx) in self.connections.iter().enumerate().skip(1) {
            let mut q = rx.write().unwrap();
            for v in self.outbox.iter() {
                if matches!(q.push((*v).clone()), Err(_)) {
                    result.mark(i);
                    break;
                }
            }
        }

        // move messages for connection 1
        if let Some(first_rx) = self.connections.get(0) {
            let mut q = first_rx.write().unwrap();
            for v in self.outbox.drain_all() {
                if matches!(q.push(v), Err(_)) {
                    result.mark(0);
                    break;
                }
            }
        } else {
            // still clear outbox if there is no connection
            self.outbox.clear();
        }

        result.into()
    }

    fn is_connected(&self) -> bool {
        !self.connections.is_empty()
    }
}

impl<T: Send + Sync + Clone> Tx for Option<DoubleBufferTx<T>> {
    fn flush(&mut self) -> Result<(), FlushError> {
        if let Some(tx) = self.as_mut() {
            tx.flush()
        } else {
            Ok(())
        }
    }

    fn is_connected(&self) -> bool {
        self.as_ref().map_or(false, |tx| tx.is_connected())
    }
}

impl<T: Send + Sync + Clone> TxBundle for DoubleBufferTx<T> {
    fn name(&self, index: usize) -> String {
        assert_eq!(index, 0);
        String::from("out")
    }

    fn flush_all(&mut self) -> Result<(), MultiFlushError> {
        self.flush().map_err(|fe| MultiFlushError(vec![(0, fe)]))
    }

    fn check_connection(&self) -> ConnectionCheck {
        let mut cc = ConnectionCheck::new(1);
        cc.mark(0, self.is_connected());
        cc
    }
}

impl<T: Send + Sync + Clone> TxBundle for Option<DoubleBufferTx<T>> {
    fn name(&self, index: usize) -> String {
        assert_eq!(index, 0);
        String::from("out")
    }

    fn flush_all(&mut self) -> Result<(), MultiFlushError> {
        if let Some(tx) = self.as_mut() {
            tx.flush().map_err(|fe| MultiFlushError(vec![(0, fe)]))
        } else {
            Ok(())
        }
    }

    fn check_connection(&self) -> ConnectionCheck {
        let mut cc = ConnectionCheck::new(1);
        cc.mark(0, self.as_ref().map_or(false, |tx| tx.is_connected()));
        cc
    }
}

#[derive(Debug)]
pub struct FlushError {
    marks: NonZeroU64,
}

impl FlushError {
    pub fn new(marks: NonZeroU64) -> Self {
        Self { marks }
    }

    pub fn has_err(&self, i: usize) -> bool {
        (self.marks.get() & (1 << i)) != 0
    }
}

impl From<FlushResult> for Result<(), FlushError> {
    fn from(value: FlushResult) -> Result<(), FlushError> {
        match NonZeroU64::new(value.marks) {
            Some(marks) => Err(FlushError::new(marks)),
            None => Ok(()),
        }
    }
}

impl fmt::Display for FlushError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "FlushError({:b})", self.marks)
    }
}

impl std::error::Error for FlushError {}

#[derive(Debug)]
struct FlushResult {
    marks: u64,
}

impl FlushResult {
    pub fn new() -> Self {
        Self { marks: 0 }
    }

    pub fn mark(&mut self, i: usize) {
        self.marks &= 1 << i;
    }
}

impl<T> DoubleBufferRx<T> {
    /// Creates a new RX channel
    /// TODO deprecate in favor of `new_auto_size`, `new_fixed`, and `new_forget`
    pub fn new(overflow_policy: OverflowPolicy, retention_policy: RetentionPolicy) -> Self {
        let back = BackStage::new(overflow_policy, retention_policy);
        let capacity = back.capacity();
        Self {
            back: Arc::new(RwLock::new(back)),
            front: FrontStage::new(capacity),
            is_connected: false,
        }
    }

    /// Creates a channel which stores the most recent message
    pub fn new_latest() -> Self {
        Self::new(OverflowPolicy::Forget(1), RetentionPolicy::Keep)
    }

    /// Creates a channel which automatically resizes itself to always succeed in receiving
    /// all messages.
    /// WARNING: This might lead to data congestion and infinitely growing queues. Usually it is
    /// better to use a fixed capacity or to forget old messages.
    pub fn new_auto_size() -> Self {
        Self::new(
            OverflowPolicy::Resize(StrictlyIncreasingLinear::from_factor(2)),
            RetentionPolicy::Drop,
        )
    }

    pub fn pop_all(&mut self) -> std::collections::vec_deque::Drain<'_, T> {
        self.front.drain(..)
    }

    /// Number of messages currently visible. Additional messages might be stored in the stage
    /// buffer.
    pub fn len(&self) -> usize {
        self.front.len()
    }

    pub fn clear(&mut self) {
        self.front.clear();
    }

    pub fn drain<R>(&mut self, range: R) -> vec_deque::Drain<'_, T>
    where
        R: ops::RangeBounds<usize>,
    {
        self.front.drain(range)
    }
}

impl<T> DoubleBufferRx<Message<T>> {
    pub fn as_acq_time_series<'a>(&'a self) -> RxChannelTimeseries<'a, T> {
        RxChannelTimeseries {
            channel: self,
            kind: TimestampKind::Acq,
        }
    }

    pub fn as_pub_time_series<'a>(&'a self) -> RxChannelTimeseries<'a, T> {
        RxChannelTimeseries {
            channel: self,
            kind: TimestampKind::Pub,
        }
    }
}

pub trait Pop {
    type Output;

    /// Returns true if the inbox is empty.
    fn is_empty(&self) -> bool;

    /// Removes the next message from the inbox
    fn pop(&mut self) -> Result<Self::Output, RxRecvError>;

    fn try_pop(&mut self) -> Option<Self::Output> {
        self.pop().ok()
    }

    fn try_pop_update<'a, 'b>(
        &'a mut self,
        other: &'b mut Option<Self::Output>,
    ) -> &'b mut Option<Self::Output> {
        match self.try_pop() {
            Some(x) => *other = Some(x),
            None => {}
        }
        other
    }
}

impl<T> Pop for DoubleBufferRx<T> {
    type Output = T;

    fn is_empty(&self) -> bool {
        self.front.is_empty()
    }

    fn pop(&mut self) -> Result<T, RxRecvError> {
        self.front.pop().ok_or(RxRecvError::QueueEmtpy)
    }
}

impl<'a, T1: Pop, T2: Pop> Pop for (&'a mut T1, &'a mut T2) {
    type Output = (<T1 as Pop>::Output, <T2 as Pop>::Output);

    fn is_empty(&self) -> bool {
        self.0.is_empty() || self.1.is_empty()
    }

    fn pop(&mut self) -> Result<Self::Output, RxRecvError> {
        if self.is_empty() {
            Err(RxRecvError::QueueEmtpy)
        } else {
            Ok((self.0.pop().unwrap(), self.1.pop().unwrap()))
        }
    }
}

impl<T> ops::Index<usize> for DoubleBufferRx<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.front[idx]
    }
}

impl<T> ops::IndexMut<usize> for DoubleBufferRx<T> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.front[idx]
    }
}

impl<T: Send + Sync> Rx for DoubleBufferRx<T> {
    fn is_connected(&self) -> bool {
        self.is_connected
    }

    fn sync(&mut self) {
        self.back.write().unwrap().sync(&mut self.front);
    }
}

impl<T: Send + Sync> Rx for Option<DoubleBufferRx<T>> {
    fn is_connected(&self) -> bool {
        self.as_ref().map_or(false, |rx| rx.is_connected)
    }

    fn sync(&mut self) {
        if let Some(rx) = self.as_mut() {
            rx.sync()
        }
    }
}

impl<T: Send + Sync> RxBundle for DoubleBufferRx<T> {
    fn name(&self, index: usize) -> String {
        assert_eq!(index, 0);
        String::from("in")
    }

    fn sync_all(&mut self) {
        self.sync()
    }

    fn check_connection(&self) -> ConnectionCheck {
        let mut cc = ConnectionCheck::new(1);
        cc.mark(0, self.is_connected());
        cc
    }
}

impl<T: Send + Sync> RxBundle for Option<DoubleBufferRx<T>> {
    fn name(&self, index: usize) -> String {
        assert_eq!(index, 0);
        String::from("in")
    }

    fn sync_all(&mut self) {
        if let Some(rx) = self.as_mut() {
            rx.sync()
        }
    }

    fn check_connection(&self) -> ConnectionCheck {
        let mut cc = ConnectionCheck::new(1);
        cc.mark(0, self.as_ref().map_or(false, |rx| rx.is_connected()));
        cc
    }
}

#[derive(Debug)]
pub enum TxSendError {
    QueueFull,
}

impl fmt::Display for TxSendError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            TxSendError::QueueFull => write!(fmt, "QueueFull"),
        }
    }
}

impl std::error::Error for TxSendError {}

#[derive(Debug)]
pub enum RxRecvError {
    QueueEmtpy,
}

impl fmt::Display for RxRecvError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            RxRecvError::QueueEmtpy => write!(fmt, "QueueEmtpy"),
        }
    }
}

impl std::error::Error for RxRecvError {}

#[cfg(test)]
mod tests {
    use crate::channels::double_buffer_channel::fixed_channel;
    use crate::prelude::*;
    use std::sync::mpsc;

    #[test]
    fn test() {
        const NUM_MESSAGES: usize = 100;
        const NUM_ROUNDS: usize = 100;

        let (mut tx, mut rx) = fixed_channel(NUM_MESSAGES);

        // channel used for synchronizing tx and rx threads
        let (sync_tx, sync_rx) = mpsc::sync_channel(1);
        let (rep_tx, rep_rx) = mpsc::sync_channel(1);

        // receiver
        let t1 = std::thread::spawn(move || {
            for k in 0..NUM_ROUNDS {
                // wait for signal to sync
                sync_rx.recv().unwrap();
                rx.sync();
                rep_tx.send(()).unwrap();

                // receive messages
                for i in 0..NUM_MESSAGES {
                    assert_eq!(rx.pop().unwrap(), format!("hello {k} {i}"));
                }
            }
        });

        // sender
        let t2 = std::thread::spawn(move || {
            for k in 0..NUM_ROUNDS {
                // send messages
                for i in 0..NUM_MESSAGES {
                    tx.push(format!("hello {k} {i}")).unwrap();
                }
                tx.flush().unwrap();

                // send sync signal
                sync_tx.send(()).unwrap();
                rep_rx.recv().unwrap();
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();
    }
}
