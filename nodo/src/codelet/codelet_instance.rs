// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::channels::{RxBundle, TxBundle};
use crate::codelet::{Codelet, Context, TaskClock, Transition, SUCCESS};
use log::error;
use nodo_core::*;

/// Named instance of a codelet with configuration and channel bundels
pub struct CodeletInstance<C: Codelet> {
    pub name: String,
    pub state: C,
    pub config: C::Config,
    pub rx: C::Rx,
    pub tx: C::Tx,

    pub(crate) clock: Option<TaskClock>,
    pub(crate) is_scheduled: bool,
}

impl<C: Codelet> Drop for CodeletInstance<C> {
    fn drop(&mut self) {
        if !self.is_scheduled {
            error!(
                "Codelet instance `{}` was created and destroyed without every being scheduled",
                self.name
            );
        }
    }
}

impl<C: Codelet> CodeletInstance<C> {
    /// Creates a new instance with given state and config
    pub(crate) fn new<S: Into<String>>(name: S, state: C, config: C::Config) -> Self {
        let (rx, tx) = C::build_bundles(&config);
        Self {
            name: name.into(),
            state,
            config,
            rx,
            tx,
            clock: None,
            is_scheduled: false,
        }
    }

    pub fn modify_state_with<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut C) -> (),
    {
        f(&mut self.state);
        self
    }

    pub fn start(&mut self) -> Outcome {
        let cc = self.rx.check_connection();
        if !cc.is_fully_connected() {
            error!(
                "codelet '{}' (type={}) has unconnected RX channels: {}",
                self.name,
                self.type_name(),
                cc.list_unconnected()
                    .iter()
                    .map(|&i| format!("[{i}] {}", self.rx.name(i)))
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        }

        let cc = self.tx.check_connection();
        if !cc.is_fully_connected() {
            error!(
                "codelet '{}' (type={}) has unconnected TX channels: {}",
                self.name,
                self.type_name(),
                cc.list_unconnected()
                    .iter()
                    .map(|&i| format!("[{i}] {}", self.tx.name(i)))
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        }

        self.rx.sync();
        self.clock.as_mut().unwrap().start();
        self.state.start(
            &Context {
                clock: &self.clock.as_ref().unwrap(),
                config: &self.config,
            },
            &mut self.rx,
            &mut self.tx,
        )?;
        self.tx.flush()?;

        SUCCESS
    }

    pub fn stop(&mut self) -> Outcome {
        self.rx.sync();
        self.state.stop(
            &Context {
                clock: &self.clock.as_ref().unwrap(),
                config: &self.config,
            },
            &mut self.rx,
            &mut self.tx,
        )?;
        self.tx.flush()?;
        SUCCESS
    }

    pub fn step(&mut self) -> Outcome {
        self.rx.sync();
        self.clock.as_mut().unwrap().step();
        self.state.step(
            &Context {
                clock: &self.clock.as_ref().unwrap(),
                config: &self.config,
            },
            &mut self.rx,
            &mut self.tx,
        )?;
        self.tx.flush()?;
        SUCCESS
    }

    pub fn pause(&mut self) -> Outcome {
        self.state.pause()
    }

    pub fn resume(&mut self) -> Outcome {
        self.state.resume()
    }
}

/// An abstract interface for `CodeletInstance` hiding the concrete codelet type
pub trait CodeletExec: Send {
    /// Called once at the beginning to setup the clock
    fn setup(&mut self, clock: TaskClock);

    /// Called to transition the state of the codelet instance
    fn execute(&mut self, transition: Transition) -> Outcome;
}

impl<C: Codelet> CodeletExec for CodeletInstance<C> {
    fn setup(&mut self, clock: TaskClock) {
        self.clock = Some(clock);
    }

    fn execute(&mut self, transition: Transition) -> Outcome {
        match transition {
            Transition::Start => self.start(),
            Transition::Step => self.step(),
            Transition::Stop => self.stop(),
            Transition::Pause => self.pause(),
            Transition::Resume => self.resume(),
        }
    }
}

/// Identification of a codelet instance
pub trait CodeletInstanceId {
    /// The name of this instance
    fn name(&self) -> &str;

    /// The typename of the codelet used by this instance
    fn type_name(&self) -> &str;
}

impl<C: Codelet> CodeletInstanceId for CodeletInstance<C> {
    fn name(&self) -> &str {
        &self.name
    }

    fn type_name(&self) -> &str {
        std::any::type_name::<C>()
    }
}