// Copyright 2024 by David Weikersdorfer. All rights reserved.

use core::future::Future;
use core::time::Duration;

/// For now a basic wrapper around a tokio runtime
pub struct AsyncRuntime {
    runtime: tokio::runtime::Runtime,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl AsyncRuntime {
    pub fn new() -> eyre::Result<Self> {
        Ok(Self {
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?,
            handles: Vec::new(),
        })
    }

    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handles.push(self.runtime.spawn(future));
    }

    pub fn block_on(&mut self) {
        for handle in self.handles.drain(..) {
            match self.runtime.block_on(handle) {
                Ok(..) => {}
                Err(err) => {
                    log::error!("task failed to complete: {err:?}")
                }
            }
        }
    }
}

/// Wrapper around tokio sleep function
pub fn sleep(duration: Duration) -> tokio::time::Sleep {
    tokio::time::sleep(duration)
}
