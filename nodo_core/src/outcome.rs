// Copyright 2023 by David Weikersdorfer. All rights reserved.

/// Result of an task
pub type EyreResult<T> = eyre::Result<T>;

pub type Report = eyre::Report;

/// Result of an task
pub type Outcome = EyreResult<()>;

/// The task completed successfully
pub const SUCCESS: Outcome = Ok(());

pub use eyre::{ensure, eyre, WrapErr};

/// Wrapper around eyre::ensure which can be used to check that two values are equal
#[macro_export]
macro_rules! ensure_eq {
    ($l:expr, $r:expr $(, { $($rest:tt)* })?) => {
        match (&$l, &$r) {
            (left, right) => {
                $crate::ensure!(*left == *right,
                	"condition failed: {:?} == {:?}",
                	&*left, &*right, $($($rest)*)?);
            }
        }
    };
    ($l:expr, $r:expr $(, $($rest:tt)*)?) => {
        match (&$l, &$r) {
            (left, right) => {
                $crate::ensure!(*left == *right $(, $($rest)*)?);
            }
        }
    };
}
