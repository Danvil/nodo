use crate::{
    channels::TxConnectError,
    prelude::{DoubleBufferRx, DoubleBufferTx},
};

/// Connects two channels together
pub fn connect<Tx, Rx>(tx: Tx, rx: Rx) -> Result<(), TxConnectError>
where
    (Tx, Rx): Connect,
{
    (tx, rx).connect()
}

/// Trait used to implement the `connect` function and provide "function overloading" for different
/// varieties of TX and RX channels.
pub trait Connect {
    fn connect(self) -> Result<(), TxConnectError>;
}

impl<T: Send + Sync> Connect for (&mut DoubleBufferTx<T>, &mut DoubleBufferRx<T>) {
    fn connect(self) -> Result<(), TxConnectError> {
        self.0.connect(self.1)
    }
}

impl<T: Send + Sync> Connect for (Option<&mut DoubleBufferTx<T>>, &mut DoubleBufferRx<T>) {
    fn connect(self) -> Result<(), TxConnectError> {
        if let Some(tx) = self.0 {
            tx.connect(self.1)
        } else {
            Ok(())
        }
    }
}

impl<T: Send + Sync> Connect for (&mut DoubleBufferTx<T>, Option<&mut DoubleBufferRx<T>>) {
    fn connect(self) -> Result<(), TxConnectError> {
        if let Some(rx) = self.1 {
            self.0.connect(rx)
        } else {
            Ok(())
        }
    }
}

impl<T: Send + Sync> Connect
    for (
        Option<&mut DoubleBufferTx<T>>,
        Option<&mut DoubleBufferRx<T>>,
    )
{
    fn connect(self) -> Result<(), TxConnectError> {
        if let (Some(tx), Some(rx)) = (self.0, self.1) {
            tx.connect(rx)
        } else {
            Ok(())
        }
    }
}
