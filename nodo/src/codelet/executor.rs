// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::ScheduleExecutor;
use crate::codelet::Statistics;
use crate::codelet::TaskClock;
use crate::sleep::accurate_sleep_until;
use nodo_core::MonotonicClock;
use nodo_core::PubtimeMarker;
use std::collections::HashMap;

pub struct Executor {
    clock: MonotonicClock<PubtimeMarker>,
    workers: Vec<Worker>,
}

pub enum WorkerRequest {
    Stop,
    Statistics,
}

pub enum WorkerReply {
    Statistics(HashMap<(String, String), Statistics>),
}

pub struct WorkerState {
    schedule: ScheduleExecutor,
    rx_request: std::sync::mpsc::Receiver<WorkerRequest>,
    tx_reply: std::sync::mpsc::Sender<WorkerReply>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            clock: MonotonicClock::new(),
            workers: Vec::new(),
        }
    }

    pub fn push(&mut self, mut schedule: ScheduleExecutor) {
        schedule.setup_task_clock(TaskClock::from(self.clock.clone()));
        self.workers.push(Worker::new(schedule));
    }

    pub fn is_finished(&self) -> bool {
        self.workers.iter().all(|w| w.is_finished())
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

    pub fn statistics(&self) -> HashMap<(String, String), Statistics> {
        let mut result = HashMap::new();
        for w in self.workers.iter() {
            result.extend(w.statistics());
        }
        result
    }
}

pub struct Worker {
    name: String,
    thread: Option<std::thread::JoinHandle<()>>,
    tx_request: std::sync::mpsc::Sender<WorkerRequest>,
    rx_reply: std::sync::mpsc::Receiver<WorkerReply>,
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
        }
    }

    fn is_finished(&self) -> bool {
        self.thread.as_ref().map_or(true, |h| h.is_finished())
    }

    fn join(&mut self) -> Result<(), ()> {
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| ())
        } else {
            Ok(())
        }
    }

    fn worker_thread(mut state: WorkerState) {
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

            // handle requests
            match state.rx_request.try_recv() {
                Ok(WorkerRequest::Stop) => break,
                Ok(WorkerRequest::Statistics) => state
                    .tx_reply
                    .send(WorkerReply::Statistics(state.schedule.statistics()))
                    .unwrap(),
                Err(_) => {
                    // FIXME
                }
            };

            // execute
            state.schedule.spin();
            if state.schedule.is_terminated() {
                break;
            }
        }

        state.schedule.finalize();

        state
            .tx_reply
            .send(WorkerReply::Statistics(state.schedule.statistics()))
            .ok();
    }

    fn statistics(&self) -> HashMap<(String, String), Statistics> {
        self.tx_request.send(WorkerRequest::Statistics).ok();
        match self.rx_reply.recv() {
            Ok(WorkerReply::Statistics(stats)) => stats,
            _ => panic!(),
        }
    }
}
