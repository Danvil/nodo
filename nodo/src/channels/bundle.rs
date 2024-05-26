// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::channels::FlushError;
use crate::channels::MAX_RECEIVER_COUNT;
use paste::paste;

/// An endpoint receiving data
pub trait Rx: Send {
    /// Prepares receiving of messages
    fn sync(&mut self);

    /// Returns true if the channel is connected
    fn is_connected(&self) -> bool;
}

/// An endpoint publishing data
pub trait Tx: Send {
    /// Finalizes sending of messages
    fn flush(&mut self) -> Result<(), FlushError>;

    /// Returns true if the channel is connected
    fn is_connected(&self) -> bool;
}

/// A collection of receiving endpoints. Synchronizing the bundle will synchronize all endpoints it
/// contains.
pub trait RxBundle: Send {
    /// Name of the i-th endpoint
    fn name(&self, index: usize) -> String;

    /// Synchronizes all endpoints
    fn sync_all(&mut self);

    /// Connection status of all endpoints in the budle
    fn check_connection(&self) -> ConnectionCheck;
}

/// A collection of transmitting endpoints. Flushing the bundle will flush all endpoints it
/// contains.
pub trait TxBundle: Send {
    /// Name of the i-th endpoint
    fn name(&self, index: usize) -> String;

    /// Flushes all endpoints
    fn flush_all(&mut self) -> Result<(), MultiFlushError>;

    /// Connection status of all endpoints in the budle
    fn check_connection(&self) -> ConnectionCheck;
}

#[derive(Debug, thiserror::Error)]
#[error("MultiFlushError({:?})", self.0)]
pub struct MultiFlushError(pub Vec<(usize, FlushError)>);

// impl From<FlushError> for MultiFlushError {
//     fn from(error: FlushError) -> Self {
//         MultiFlushError(vec![(0, error)])
//     }
// }

macro_rules! count {
    () => (0usize);
    ($x:tt $($xs:tt)*) => (1usize + count!($($xs)*));
}

impl RxBundle for () {
    fn name(&self, _index: usize) -> String {
        panic!("empty bundle")
    }

    fn sync_all(&mut self) {}

    fn check_connection(&self) -> ConnectionCheck {
        ConnectionCheck::default()
    }
}

macro_rules! impl_rx_bundle_tuple {
    ( $( $ty: ident, $i: literal ),* ) => {
        impl<$($ty),*> RxBundle for ($($ty,)*) where $($ty: Rx,)* {
            fn name(&self, index: usize) -> String {
                let len = count!($($ty)*);
                assert!(index < len);
                format!("{index}")
            }

            fn sync_all(&mut self) {
                $(paste!{self.$i}.sync();)*
            }

            fn check_connection(&self) -> ConnectionCheck {
                let len = count!($($ty)*);
                let mut cc = ConnectionCheck::new(len);
                $(cc.mark($i, paste!{self.$i}.is_connected());)*
                cc
            }
        }
    };
}

impl_rx_bundle_tuple!(A, 0);
impl_rx_bundle_tuple!(A, 0, B, 1);
impl_rx_bundle_tuple!(A, 0, B, 1, C, 2);
impl_rx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3);
impl_rx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4);
impl_rx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4, F, 5);
impl_rx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6);
impl_rx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7);

impl TxBundle for () {
    fn name(&self, _index: usize) -> String {
        panic!("empty bundle")
    }

    fn flush_all(&mut self) -> Result<(), MultiFlushError> {
        Ok(())
    }

    fn check_connection(&self) -> ConnectionCheck {
        ConnectionCheck::default()
    }
}

macro_rules! impl_tx_bundle_tuple {
    ( $( $ty: ident, $i: literal ),* ) => {
        impl<$($ty),*> TxBundle for ($($ty,)*) where $($ty: Tx,)* {
            fn name(&self, index: usize) -> String {
                let len = count!($($ty)*);
                assert!(index < len);
                format!("{index}")
            }

            fn flush_all(&mut self) -> Result<(), MultiFlushError> {
                let errs: Vec<(usize, FlushError)> = [$(paste!{self.$i}.flush(),)*]
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, res)| {
                        if let Some(err) = res.err() {
                            Some((i, err))
                        } else {
                            None
                        }
                    })
                    .collect();
                if errs.is_empty() {
                    Ok(())
                } else {
                    Err(MultiFlushError(errs))
                }
            }

            fn check_connection(&self) -> ConnectionCheck {
                let len = count!($($ty)*);
                let mut cc = ConnectionCheck::new(len);
                $(cc.mark($i, paste!{self.$i}.is_connected());)*
                cc
            }
        }
    };
}

impl_tx_bundle_tuple!(A, 0);
impl_tx_bundle_tuple!(A, 0, B, 1);
impl_tx_bundle_tuple!(A, 0, B, 1, C, 2);
impl_tx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3);
impl_tx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4);
impl_tx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4, F, 5);
impl_tx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6);
impl_tx_bundle_tuple!(A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7);

/// A collection of boolean flags indicating if an endpoint is connected.
#[derive(Debug)]
pub struct ConnectionCheck(u8, u64);

impl Default for ConnectionCheck {
    fn default() -> Self {
        Self(0, 0)
    }
}

impl ConnectionCheck {
    pub fn new(len: usize) -> Self {
        assert!(len <= MAX_RECEIVER_COUNT, "too many connections: len={len}");

        Self(len as u8, 0)
    }

    /// Sets the connections status of a channel
    pub fn mark(&mut self, index: usize, is_connected: bool) {
        assert!(
            index < self.0.into(),
            "invalid channel index: len={}, index={}",
            self.0,
            index
        );

        if is_connected {
            self.1 |= 1 << index
        } else {
            self.1 &= !(1 << index)
        }
    }

    /// Returns true if the channel with given index is connected
    pub fn is_connected(&self, index: usize) -> bool {
        assert!(
            index < self.0.into(),
            "invalid channel index: len={}, index={}",
            self.0,
            index
        );

        self.1 & (1 << index) != 0
    }

    /// Returns true if all endpoints are connected
    pub fn is_fully_connected(&self) -> bool {
        // FIXME I will never know how to safely create a mask with first N bits set...
        for i in 0..self.0 as usize {
            if !self.is_connected(i) {
                return false;
            }
        }
        true
    }

    /// Gets the indices of all unconnected endpoints
    pub fn list_unconnected(&self) -> Vec<usize> {
        (0..self.0 as usize)
            .filter(|&i| !self.is_connected(i))
            .collect()
    }
}
