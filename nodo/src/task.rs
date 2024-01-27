// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::future::Future;
use futures::future::BoxFuture;
use futures::task::ArcWake;
use futures::FutureExt;
use nodo_core::Outcome;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::sync::Mutex;

// /// An task which can be executed asynchronously
// #[async_trait]
// pub trait Task: Send + Sync {
//     async fn run(&mut self) -> Outcome;
// }

/// A future that can reschedule itself to be polled by an `Executor`.
pub struct Task {
    /// In-progress future that should be pushed to completion.
    /// BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>
    future: Mutex<Option<BoxFuture<'static, Outcome>>>,

    /// Handle to place the task itself back onto the task queue.
    task_sender: SyncSender<Arc<Task>>,
}

impl Task {
    pub fn new(
        sender: SyncSender<Arc<Task>>,
        future: impl Future<Output = Outcome> + 'static + Send,
    ) -> Arc<Self> {
        let task = Arc::new(Self {
            future: Mutex::new(Some(future.boxed())),
            task_sender: sender.clone(),
        });
        sender.send(task.clone()).expect("too many tasks queued");
        task
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Implement `wake` by sending this task back onto the task channel
        // so that it will be polled again by the executor.
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("too many tasks queued");
    }
}
