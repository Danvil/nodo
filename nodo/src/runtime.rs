// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::statistics_pretty_print;
use crate::codelet::Executor as CodeletExecutor;
use crate::codelet::ScheduleExecutor as CodeletSchedule;
use crate::task::Task;
use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use futures::task::waker_ref;
use futures::{future::FutureExt, task::ArcWake};
use std::sync::Arc;

pub struct Runtime {
    tx_spawn: std::sync::mpsc::SyncSender<Arc<Task>>,
    rx_spawn: std::sync::mpsc::Receiver<Arc<Task>>,
    codelet_exec: CodeletExecutor,
}

pub struct DummyTask;

impl ArcWake for DummyTask {
    fn wake_by_ref(_arc_self: &Arc<Self>) {}
}

type TaskBoxFuture = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

impl Runtime {
    pub fn new() -> Self {
        let (tx_spawn, rx_spawn) = std::sync::mpsc::sync_channel(16);
        let codelet_exec = CodeletExecutor::new();
        Self {
            tx_spawn,
            rx_spawn,
            codelet_exec,
        }
    }

    pub fn block_on<F: Future + Send>(&self, f: F) -> Result<F::Output, ()> {
        let mut fbox = f.boxed();
        loop {
            let task = Arc::new(DummyTask);
            let waker = waker_ref(&task);
            let mut context = Context::from_waker(&waker);
            match fbox.as_mut().poll(&mut context) {
                Poll::Ready(x) => return Ok(x),
                Poll::Pending => {}
            }
        }
    }

    pub fn add_codelet_schedule(&mut self, schedule: CodeletSchedule) {
        self.codelet_exec.push(schedule)
    }

    pub fn request_stop(&mut self) {
        self.codelet_exec.request_stop();
    }

    /// Installs a signal handler, waits until Ctrl+C is pressed, and then stops all execution.
    pub fn wait_for_ctrl_c(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();

        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
            .expect("Error setting Ctrl-C handler");

        log::warn!("Executing until Ctrl+C is pressed..");
        rx.recv().expect("Could not receive from channel.");

        log::warn!("Received Ctrl+C! Requesting stop and waiting for workers to finish..");
        self.request_stop();
        self.join().unwrap();
        log::info!("All workers stopped.");
    }

    pub fn spawn<T: 'static>(&mut self, task: T) {
        // self.tx_spawn.send(Box::new(task));
    }

    pub fn join(&mut self) -> Result<(), ()> {
        self.codelet_exec.join();
        statistics_pretty_print(self.codelet_exec.statistics());
        Ok(())
        // let mut tasks: Vec<Arc<RwLock<Box<dyn Task>>>> = Vec::new();
        // let mut futures: Vec<TaskBoxFuture> = Vec::new();

        // loop {
        //     // schedule codelets
        //     self.codelet_scheduler.step()?;

        //     // receive new tasks
        //     while let Ok(task) = self.rx_spawn.try_recv() {
        //         let arc = Arc::new(RwLock::new(task));
        //         // futures.push(arc.write().unwrap().run().boxed());
        //         tasks.push(arc);
        //     }

        //     let mut next_futures = Vec::new();
        //     for mut future in futures.into_iter() {
        //         let dummy = Arc::new(DummyTask);
        //         let waker = waker_ref(&dummy);
        //         let mut context = Context::from_waker(&waker);
        //         match future.as_mut().poll(&mut context) {
        //             Poll::Ready(_x) => {}
        //             Poll::Pending => next_futures.push(future),
        //         }
        //     }
        //     futures = next_futures;
        // }
    }
}
