// Copyright 2023 by David Weikersdorfer. All rights reserved.

/// Result of an task
pub type EyreResult<T> = eyre::Result<T>;

pub type Report = eyre::Report;

/// Result of an task
pub type Outcome = EyreResult<()>;

/// The task completed successfully
pub const SUCCESS: Outcome = Ok(());

pub use eyre::{ensure, eyre, WrapErr};
