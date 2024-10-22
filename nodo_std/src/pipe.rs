use core::marker::PhantomData;
use nodo::prelude::*;

// FIXME replace nodo::Pipe with this one
pub struct Pipe<T, S, F> {
    callback: F,
    marker: PhantomData<(T, S)>,
}

pub enum PipeConfig {
    OneToOne,
    Dynamic,
}

impl<T, S, F> Pipe<T, S, F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            marker: PhantomData,
        }
    }
}

impl<T, S, F> Codelet for Pipe<T, S, F>
where
    T: Send + Sync,
    S: Clone + Send + Sync,
    F: FnMut(T) -> S + Send,
{
    type Status = DefaultStatus;
    type Config = PipeConfig;
    type Rx = DoubleBufferRx<T>;
    type Tx = DoubleBufferTx<S>;

    fn build_bundles(config: &Self::Config) -> (Self::Rx, Self::Tx) {
        match config {
            PipeConfig::OneToOne => (
                DoubleBufferRx::new(OverflowPolicy::Reject(1), RetentionPolicy::EnforceEmpty),
                DoubleBufferTx::new(1),
            ),
            PipeConfig::Dynamic => (
                DoubleBufferRx::new_auto_size(),
                DoubleBufferTx::new_auto_size(),
            ),
        }
    }

    fn start(&mut self, _cx: &Context<Self>, rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        // FIXME There is a general problem in nodo as messages can be received during start, and
        //       if they are not handled EnforceEmpty will panic.
        //       In the wild only this codelet seem to have this problem so we fix it here for now.
        while let Some(_) = rx.try_pop() {}
        SUCCESS
    }

    fn step(&mut self, ctx: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        match ctx.config {
            PipeConfig::OneToOne => {
                if let Some(msg) = rx.try_pop() {
                    tx.push((self.callback)(msg))?;
                    SUCCESS
                } else {
                    SKIPPED
                }
            }
            PipeConfig::Dynamic => {
                let skipped = rx.is_empty();
                while let Some(msg) = rx.try_pop() {
                    tx.push((self.callback)(msg))?;
                }
                if skipped {
                    SKIPPED
                } else {
                    SUCCESS
                }
            }
        }
    }
}
