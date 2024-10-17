// Copyright 2023 by David Weikersdorfer. All rights reserved.

pub use eyre::{ensure, eyre, Result, WrapErr};

/// Result of an task
pub type EyreResult<T> = eyre::Result<T>;

pub type Report = eyre::Report;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultStatus {
    /// The codelet skipped this step as there was no work to do.
    /// Skipped steps are counted separately in statistics and other tools.
    Skipped,

    /// The codelet executed work and is still running.
    Running,
}

pub const SKIPPED: Outcome = Ok(DefaultStatus::Skipped);

// TODO to be enabled #[deprecated(note = "use RUNNING instead")]
pub const SUCCESS: Outcome = Ok(DefaultStatus::Running);
pub const RUNNING: Outcome = Ok(DefaultStatus::Running);

/// Result of an task
// TODO to be deprecated
pub type Outcome = Result<OutcomeKind>;

// TODO to be deprecated
pub type OutcomeKind = DefaultStatus;
