// Copyright 2023 by David Weikersdorfer. All rights reserved.

pub use eyre::{ensure, eyre, WrapErr};

/// Result of an task
pub type EyreResult<T> = eyre::Result<T>;

pub type Report = eyre::Report;

/// Result of an task
pub type Outcome = Result<OutcomeKind, eyre::Report>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeKind {
    /// The codelet skipped this step as there was no work to do.
    /// Skipped steps are counted separately in statistics and other tools.
    Skipped,

    /// The codelet executed work and is still running.
    Running,

    /// The codelet terminated successfully and does not need to be run again.
    /// If a codelet returns Terminated it's stop function will be called, and it will not step
    /// again unless explicitely started again.
    Terminated,
}

pub const SKIPPED: Outcome = Ok(OutcomeKind::Skipped);

// TODO to be enabled #[deprecated(note = "use RUNNING instead")]
pub const SUCCESS: Outcome = Ok(OutcomeKind::Running);
pub const RUNNING: Outcome = Ok(OutcomeKind::Running);

pub const TERMINATED: Outcome = Ok(OutcomeKind::Terminated);
