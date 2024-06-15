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
