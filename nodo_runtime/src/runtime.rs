// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::{
    statistics_pretty_print, Executor as CodeletExecutor, ScheduleExecutor as CodeletSchedule,
};
use core::time::Duration;
use nodo::prelude::RuntimeControl;
use std::sync::mpsc::RecvTimeoutError;

pub struct Runtime {
    tx_control: std::sync::mpsc::SyncSender<RuntimeControl>,
    rx_control: std::sync::mpsc::Receiver<RuntimeControl>,
    codelet_exec: CodeletExecutor,
}

impl Runtime {
    pub fn new() -> Self {
        let (tx_control, rx_control) = std::sync::mpsc::sync_channel(16);
        let codelet_exec = CodeletExecutor::new();
        Self {
            tx_control,
            rx_control,
            codelet_exec,
        }
    }

    pub fn add_codelet_schedule(&mut self, schedule: CodeletSchedule) {
        self.codelet_exec.push(schedule)
    }

    pub fn tx_control(&mut self) -> std::sync::mpsc::SyncSender<RuntimeControl> {
        self.tx_control.clone()
    }

    /// If called the program will stop when Ctrl+C is pressed
    pub fn enable_terminate_on_ctrl_c(&mut self) {
        log::info!("Press Ctrl+C to stop..");

        let tx = self.tx_control();
        ctrlc::set_handler(move || {
            tx.send(RuntimeControl::RequestStop)
                .expect("Could not send signal on channel.")
        })
        .expect("Error setting Ctrl-C handler");
    }

    pub fn spin(&mut self) {
        let sleep_duration = Duration::from_millis(500);

        loop {
            match self.rx_control.recv_timeout(sleep_duration) {
                Err(RecvTimeoutError::Timeout) => {
                    if self.codelet_exec.is_finished() {
                        log::info!("All workers finished.");
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("control channel disconnected");
                }
                Ok(RuntimeControl::RequestStop) => {
                    log::info!("Stop requested..");
                    self.codelet_exec.request_stop();
                    self.codelet_exec.join();
                    log::info!("All workers stopped.");
                    break;
                }
            }
        }

        statistics_pretty_print(self.codelet_exec.statistics());
    }

    #[deprecated(since = "0.2.0", note = "use `enable_terminate_on_ctrl_c` instead")]
    pub fn wait_for_ctrl_c(&mut self) {
        self.enable_terminate_on_ctrl_c();
        self.spin();
    }
}
