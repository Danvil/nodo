// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::statistics_pretty_print;
use crate::codelet::Executor;
use crate::codelet::Manifold;
use crate::codelet::ScheduleExecutor;
use crate::inspector::Inspector;
use crate::task::Task;
use core::future::Future;
use core::task::Context;
use core::task::Poll;
use core::time::Duration;
use futures::task::waker_ref;
use futures::{future::FutureExt, task::ArcWake};
use std::sync::Arc;

pub struct Runtime {
    tx_control: std::sync::mpsc::SyncSender<RuntimeControl>,
    rx_control: std::sync::mpsc::Receiver<RuntimeControl>,
    tx_spawn: std::sync::mpsc::SyncSender<Arc<Task>>,
    rx_spawn: std::sync::mpsc::Receiver<Arc<Task>>,
    executor: Executor,
    manifold: Manifold,
    inspector: Inspector,
}

pub struct DummyTask;

impl ArcWake for DummyTask {
    fn wake_by_ref(_arc_self: &Arc<Self>) {}
}

#[derive(Debug, Clone, Copy)]
pub enum RuntimeControl {
    /// Request the runtime to stop. It may take a while for the runtime to shut down as codelets
    /// will finish stepping and stop will be called for all active codelets.
    RequestStop,
}

impl Runtime {
    pub fn new() -> Self {
        let (tx_control, rx_control) = std::sync::mpsc::sync_channel(16);
        let (tx_spawn, rx_spawn) = std::sync::mpsc::sync_channel(16);
        let executor = Executor::new();
        Self {
            tx_control,
            rx_control,
            tx_spawn,
            rx_spawn,
            manifold: Manifold::new(),
            executor,
            inspector: Inspector::open("tcp://localhost:12345").unwrap(),
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

    pub fn add_codelet_schedule(&mut self, schedule: ScheduleExecutor) {
        self.executor.push(schedule)
    }

    pub fn request_stop(&mut self) {
        self.executor.request_stop();
    }

    pub fn tx_control(&mut self) -> std::sync::mpsc::SyncSender<RuntimeControl> {
        self.tx_control.clone()
    }

    pub fn spin(&mut self) {
        loop {
            match self.rx_control.recv_timeout(Duration::from_millis(1000)) {
                Ok(RuntimeControl::RequestStop) => {
                    log::warn!("Stop requested: Stopping all workers..");
                    self.request_stop();
                    self.join().unwrap();
                    log::info!("All workers stopped.");
                    break;
                }
                Err(_) => {
                    // keep going
                }
            }

            self.executor.step();

            if let Some(report) = self.executor.report() {
                self.inspector.send(report);
            }
        }
    }

    /// Installs a signal handler, waits until Ctrl+C is pressed, and then stops all execution.
    pub fn wait_for_ctrl_c(&mut self) {
        let tx = self.tx_control();
        ctrlc::set_handler(move || {
            tx.send(RuntimeControl::RequestStop)
                .expect("Could not send signal on channel.")
        })
        .expect("Error setting Ctrl-C handler");

        log::warn!("Executing until Ctrl+C is pressed..");
        self.spin();
    }

    pub fn spawn<T: 'static>(&mut self, _task: T) {
        // self.tx_spawn.send(Box::new(task));
    }

    pub fn join(&mut self) -> Result<(), ()> {
        self.executor.join();
        if let Some(report) = self.executor.report() {
            statistics_pretty_print(&self.manifold, &report.statistics);
        }
        Ok(())
    }
}
