// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::{
    channels::{FlushResult, RxBundle, SyncResult, TxBundle},
    codelet::{Codelet, Context, Lifecycle, TaskClocks, Transition},
};
use nodo_core::*;

/// Named instance of a codelet with configuration and channel bundels
pub struct CodeletInstance<C: Codelet> {
    pub name: String,
    pub state: C,
    pub config: C::Config,
    pub rx: C::Rx,
    pub tx: C::Tx,

    pub(crate) clocks: Option<TaskClocks>,
    pub(crate) is_scheduled: bool,
    pub(crate) rx_sync_results: Vec<SyncResult>,
    pub(crate) tx_flush_results: Vec<FlushResult>,
}

impl<C: Codelet> Drop for CodeletInstance<C> {
    fn drop(&mut self) {
        if !self.is_scheduled {
            log::warn!(
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
        let rx_count = rx.len();
        let tx_count = tx.len();
        Self {
            name: name.into(),
            state,
            config,
            rx,
            tx,
            clocks: None,
            is_scheduled: false,
            rx_sync_results: vec![SyncResult::ZERO; rx_count],
            tx_flush_results: vec![FlushResult::ZERO; tx_count],
        }
    }

    pub fn type_name(&self) -> &str {
        std::any::type_name::<C>()
    }

    pub fn modify_state_with<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut C) -> (),
    {
        f(&mut self.state);
        self
    }

    pub fn start(&mut self) -> Outcome {
        profiling::scope!(&format!("{}_start", self.name));

        log::trace!("'{}' start begin", self.name);

        let cc = self.rx.check_connection();
        if !cc.is_fully_connected() {
            log::warn!(
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
            log::warn!(
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

        self.sync()?;

        self.clocks.as_mut().unwrap().on_codelet_start();

        let outcome = self.state.start(
            &Context {
                clock: &self.clocks.as_ref().unwrap().deprecated_task_clock,
                clocks: &self.clocks.as_ref().unwrap(),
                config: &self.config,
            },
            &mut self.rx,
            &mut self.tx,
        )?;

        self.flush()?;

        log::trace!("'{}' start end ({outcome:?})", self.name);
        Ok(outcome)
    }

    pub fn stop(&mut self) -> Outcome {
        profiling::scope!(&format!("{}_stop", self.name));
        log::trace!("'{}' stop begin", self.name);

        self.sync()?;

        self.clocks.as_mut().unwrap().on_codelet_stop();

        let outcome = self.state.stop(
            &Context {
                clock: &self.clocks.as_ref().unwrap().deprecated_task_clock,
                clocks: &self.clocks.as_ref().unwrap(),
                config: &self.config,
            },
            &mut self.rx,
            &mut self.tx,
        )?;

        self.flush()?;

        log::trace!("'{}' stop end ({outcome:?})", self.name);
        Ok(outcome)
    }

    pub fn step(&mut self) -> Outcome {
        profiling::scope!(&format!("{}_step", self.name));
        log::trace!("'{}' step begin", self.name);

        self.sync()?;

        self.clocks.as_mut().unwrap().on_codelet_step();

        let outcome = self.state.step(
            &Context {
                clock: &self.clocks.as_ref().unwrap().deprecated_task_clock,
                clocks: &self.clocks.as_ref().unwrap(),
                config: &self.config,
            },
            &mut self.rx,
            &mut self.tx,
        )?;

        self.flush()?;

        log::trace!("'{}' step end ({outcome:?})", self.name);
        Ok(outcome)
    }

    pub fn pause(&mut self) -> Outcome {
        self.state.pause()
    }

    pub fn resume(&mut self) -> Outcome {
        self.state.resume()
    }

    fn sync(&mut self) -> Result<(), eyre::Report> {
        // For some codelets the TX channel count might change dynamically
        self.rx_sync_results.resize(self.rx.len(), SyncResult::ZERO);

        self.rx.sync_all(self.rx_sync_results.as_mut_slice());

        for result in self.rx_sync_results.iter() {
            if result.enforce_empty_violation {
                return Err(eyre!("'{}': sync error (EnforceEmpty violated)", self.name,));
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), eyre::Report> {
        // For some codelets the TX channel count might change dynamically
        self.tx_flush_results
            .resize(self.tx.len(), FlushResult::ZERO);

        self.tx.flush_all(self.tx_flush_results.as_mut_slice());

        for result in self.tx_flush_results.iter() {
            if result.error_indicator.is_err() {
                return Err(eyre!(
                    "'{}': flush error {}",
                    self.name,
                    result.error_indicator
                ));
            }
        }

        Ok(())
    }
}

impl<C: Codelet> Lifecycle for CodeletInstance<C> {
    fn cycle(&mut self, transition: Transition) -> Outcome {
        match transition {
            Transition::Start => self.start(),
            Transition::Step => self.step(),
            Transition::Stop => self.stop(),
            Transition::Pause => self.pause(),
            Transition::Resume => self.resume(),
        }
    }
}
