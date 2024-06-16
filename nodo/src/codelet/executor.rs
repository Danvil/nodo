// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::ScheduleExecutor;
use crate::codelet::Statistics;
use crate::codelet::TaskClock;
use crate::sleep::accurate_sleep_until;
use core::time::Duration;
use nodo_core::MonotonicClock;
use nodo_core::PubtimeMarker;
use std::collections::HashMap;
use std::time::Instant;

const REPORT_RATE: Duration = Duration::from_millis(1000);

pub struct Executor {
    clock: MonotonicClock<PubtimeMarker>,
    workers: Vec<Worker>,
    latest_report: Option<WorkerReport>,
}

pub struct Worker {
    name: String,
    thread: Option<std::thread::JoinHandle<()>>,
    tx_request: std::sync::mpsc::Sender<WorkerRequest>,
    rx_reply: std::sync::mpsc::Receiver<WorkerReply>,
    latest_report: Option<WorkerReport>,
}

pub enum WorkerRequest {
    Stop,
    Report,
}

pub enum WorkerReply {
    Report(WorkerReport),
}

pub struct WorkerState {
    schedule: ScheduleExecutor,
    rx_request: std::sync::mpsc::Receiver<WorkerRequest>,
    tx_reply: std::sync::mpsc::Sender<WorkerReply>,
}

impl WorkerState {
    pub fn report(&self) -> WorkerReport {
        WorkerReport {
            statistics: self.schedule.statistics(),
            codelets: HashMap::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct WorkerReport {
    pub statistics: HashMap<(String, String), Statistics>,
    pub codelets: HashMap<String, ()>,
}

impl WorkerReport {
    pub fn extend(&mut self, other: &WorkerReport) {
        self.statistics.extend(other.statistics.clone());
        self.codelets.extend(other.codelets.clone());
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {
            clock: MonotonicClock::new(),
            workers: Vec::new(),
            latest_report: None,
        }
    }

    pub fn report(&self) -> Option<&WorkerReport> {
        self.latest_report.as_ref()
    }

    pub fn push(&mut self, mut schedule: ScheduleExecutor) {
        schedule.setup(TaskClock::from(self.clock.clone()));
        self.workers.push(Worker::new(schedule));
    }

    pub fn join(&mut self) {
        for w in self.workers.iter_mut() {
            w.join()
                .map_err(|err| {
                    log::error!(
                        "Could not join thread of worker '{}': {err:?}. Maybe it panicked previously.",
                        w.name
                    )
                })
                .ok();
        }
    }

    pub fn request_stop(&mut self) {
        for w in self.workers.iter() {
            w.tx_request
                .send(WorkerRequest::Stop)
                .map_err(|err| {
                    log::error!(
                        "Could not request worker '{}' to stop: {err:?}. Maybe it panicked previously.",
                        w.name
                    )
                })
                .ok();
        }
    }

    pub fn step(&mut self) {
        for w in self.workers.iter_mut() {
            w.step();
        }

        let mut result = WorkerReport::default();
        for w in self.workers.iter() {
            if let Some(report) = &w.latest_report {
                result.extend(report);
            }
        }
        self.latest_report = Some(result);
    }
}

impl Worker {
    fn new(schedule: ScheduleExecutor) -> Self {
        let (tx_request, rx_request) = std::sync::mpsc::channel();
        let (tx_reply, rx_reply) = std::sync::mpsc::channel();
        let name = schedule.name().to_string();
        let state = WorkerState {
            schedule,
            rx_request,
            tx_reply,
        };
        Self {
            name: name.clone(),
            thread: Some(
                std::thread::Builder::new()
                    .name(name)
                    .spawn(move || Self::worker_thread(state))
                    .unwrap(),
            ),
            tx_request,
            rx_reply,
            latest_report: None,
        }
    }

    fn join(&mut self) -> Result<(), ()> {
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| ())
        } else {
            Ok(())
        }
    }

    fn worker_thread(mut state: WorkerState) {
        let mut latest_report_instant = Instant::now();

        loop {
            // Wait until next period. Be careful not to hold a lock on state while sleeping.
            let maybe_next_instant = {
                if let Some(period) = state.schedule.period() {
                    state.schedule.last_instant().map(|t| t + period)
                } else {
                    None
                }
            };
            if let Some(next_instant) = maybe_next_instant {
                accurate_sleep_until(next_instant);
            }

            // execute
            state.schedule.spin();
            if state.schedule.is_terminated() {
                break;
            }

            // handle requests
            match state.rx_request.try_recv() {
                Ok(WorkerRequest::Stop) => break,
                Ok(WorkerRequest::Report) => state
                    .tx_reply
                    .send(WorkerReply::Report(state.report()))
                    .unwrap(),
                Err(_) => {
                    // FIXME
                }
            };

            let now = Instant::now();
            if now - latest_report_instant > REPORT_RATE {
                latest_report_instant = now;
                state
                    .tx_reply
                    .send(WorkerReply::Report(state.report()))
                    .unwrap();
            }
        }

        state.schedule.finalize();

        state
            .tx_reply
            .send(WorkerReply::Report(state.report()))
            .ok();
    }

    fn step(&mut self) {
        while let Ok(msg) = self.rx_reply.try_recv() {
            match msg {
                WorkerReply::Report(report) => self.latest_report = Some(report),
            }
        }
    }
}
