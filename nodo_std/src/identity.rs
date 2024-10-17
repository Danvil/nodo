use core::marker::PhantomData;
use nodo::prelude::*;

/// Forwards messages as is
pub struct Identity<T> {
    marker: PhantomData<T>,
}

impl<T> Default for Identity<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T> Codelet for Identity<T>
where
    T: Clone + Send + Sync,
{
    type Status = DefaultStatus;
    type Config = ();
    type Rx = DoubleBufferRx<T>;
    type Tx = DoubleBufferTx<T>;

    fn build_bundles(_config: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new_auto_size(),
            DoubleBufferTx::new_auto_size(),
        )
    }

    fn start(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        if rx.is_empty() {
            SKIPPED
        } else {
            while let Some(msg) = rx.try_pop() {
                tx.push(msg)?;
            }
            SUCCESS
        }
    }

    fn step(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        if rx.is_empty() {
            SKIPPED
        } else {
            while let Some(msg) = rx.try_pop() {
                tx.push(msg)?;
            }
            SUCCESS
        }
    }
}
