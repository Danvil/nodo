// Copyright 2023 by David Weikersdorfer. All rights reserved.

mod codelet_instance;
mod executor;
mod schedule;
mod sequence;
mod state_machine;
mod statistics;
mod task_clock;
mod transition;
mod vise;

pub use codelet_instance::*;
pub use executor::*;
pub use schedule::*;
pub use sequence::*;
pub use state_machine::*;
pub use statistics::*;
pub use task_clock::*;
pub use transition::*;
pub use vise::*;

use crate::channels::{RxBundle, TxBundle};
use eyre::Result;
use nodo_core::DefaultStatus;

/// Codelets can be implemented by the user to execute work.
pub trait Codelet: Send {
    /// Status code used to indicate health of codelet
    type Status: CodeletStatus;

    /// Type used for configuration
    type Config: Send;

    /// Type holding all receiving (RX) endpoints
    type Rx: RxBundle;

    /// Type holding all transmitting (TX) endpoints
    type Tx: TxBundle;

    /// Constructs channel bundles
    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx);

    /// Start is guaranteed to be called first. Start may be called again after stop was called.
    fn start(
        &mut self,
        _cx: &Context<Self>,
        _rx: &mut Self::Rx,
        _tx: &mut Self::Tx,
    ) -> Result<Self::Status> {
        Ok(Self::Status::default_implementation_status())
    }

    /// Stop is guaranteed to be called at the end if start was called.
    fn stop(
        &mut self,
        _cx: &Context<Self>,
        _rx: &mut Self::Rx,
        _tx: &mut Self::Tx,
    ) -> Result<Self::Status> {
        Ok(Self::Status::default_implementation_status())
    }

    /// Step is executed periodically after the codelet is started and while it is not paused.
    fn step(
        &mut self,
        _cx: &Context<Self>,
        _rx: &mut Self::Rx,
        _tx: &mut Self::Tx,
    ) -> Result<Self::Status> {
        Ok(Self::Status::default_implementation_status())
    }

    /// Pause may be called to suspend stepping.
    fn pause(&mut self) -> Result<Self::Status> {
        Ok(Self::Status::default_implementation_status())
    }

    /// Resume is called to resume stepping. Note that stop may also be called while the codelet
    /// is paused to stop the codelet completely instead of resuming stepping.
    fn resume(&mut self) -> Result<Self::Status> {
        Ok(Self::Status::default_implementation_status())
    }
}

pub trait CodeletStatus: 'static + Send + Sync {
    /// The status used for codelet functions which have not been implemented by the user
    fn default_implementation_status() -> Self
    where
        Self: Sized;

    /// Converts the status to a default status used internally by the framework
    fn as_default_status(&self) -> DefaultStatus;

    /// A textual rendering of the status code
    fn label(&self) -> &str;
}

impl CodeletStatus for DefaultStatus {
    fn default_implementation_status() -> Self {
        DefaultStatus::Skipped
    }

    fn as_default_status(&self) -> DefaultStatus {
        *self
    }

    fn label(&self) -> &str {
        match self {
            DefaultStatus::Skipped => "skipped",
            DefaultStatus::Running => "running",
        }
    }
}

/// Context argument used for `Codelet` start, step and stop functions
pub struct Context<'a, C>
where
    C: Codelet + ?Sized,
{
    /// The instance clock provides timings of the default clock specific to this instance.
    #[deprecated(note = "Use clocks instead")]
    pub clock: &'a TaskClock,

    /// Access to various clocks
    pub clocks: &'a TaskClocks,

    /// The configuration used for this instance
    pub config: &'a C::Config,
}

/// All instances of codelets can be converted into a CodeletInstance with into_instance
///
/// ```
/// use nodo::prelude::*;
///
/// struct MyCodelet { num: u32 };
///
/// impl Codelet for MyCodelet {
///   type Status = DefaultStatus;
///   type Config = ();
///   type Rx = ();
///   type Tx = ();
///   fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) { ((),()) }
/// }
///
/// let c = MyCodelet{ num: 42 }.into_instance("my_name", ());
/// ```
pub trait IntoInstance: Codelet + Sized {
    fn into_instance<S: Into<String>>(self, name: S, config: Self::Config)
        -> CodeletInstance<Self>;
}

impl<C> IntoInstance for C
where
    C: Codelet,
{
    fn into_instance<S: Into<String>>(
        self,
        name: S,
        config: Self::Config,
    ) -> CodeletInstance<Self> {
        CodeletInstance::new(name, self, config)
    }
}

/// Default-constructible codelets can be instantiated directly
///
/// ```
/// use nodo::prelude::*;
///
/// #[derive(Default)]
/// struct MyCodelet { text: String };
///
/// impl Codelet for MyCodelet {
///   type Status = DefaultStatus;
///   type Config = ();
///   type Rx = ();
///   type Tx = ();
///   fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) { ((),()) }
/// }
///
/// let c = MyCodelet::instantiate("my_name", ());
/// ```
pub trait Instantiate: Codelet + Sized {
    fn instantiate<S: Into<String>>(name: S, config: Self::Config) -> CodeletInstance<Self>;
}

impl<C> Instantiate for C
where
    C: Codelet + Default,
{
    fn instantiate<S: Into<String>>(name: S, config: Self::Config) -> CodeletInstance<Self> {
        CodeletInstance::new(name, C::default(), config)
    }
}
