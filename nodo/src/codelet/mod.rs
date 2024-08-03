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
use nodo_core::{Outcome, SKIPPED};

/// Codelets can be implemented by the user to execute work.
pub trait Codelet: Send {
    /// Type used for configuration
    type Config: Send;

    /// Type holding all receiving (RX) endpoints
    type Rx: RxBundle;

    /// Type holding all transmitting (TX) endpoints
    type Tx: TxBundle;

    /// Constructs channel bundles
    fn build_bundles(cfg: &Self::Config) -> (Self::Rx, Self::Tx);

    /// Start is guaranteed to be called first. Start may be called again after stop was called.
    fn start(&mut self, _cx: &Context<Self>, _rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        SKIPPED
    }

    /// Stop is guaranteed to be called at the end if start was called.
    fn stop(&mut self, _cx: &Context<Self>, _rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        SKIPPED
    }

    /// Step is executed periodically after the codelet is started and while it is not paused.
    fn step(&mut self, _cx: &Context<Self>, _rx: &mut Self::Rx, _tx: &mut Self::Tx) -> Outcome {
        SKIPPED
    }

    /// Pause may be called to suspend stepping.
    fn pause(&mut self) -> Outcome {
        SKIPPED
    }

    /// Resume is called to resume stepping. Note that stop may also be called while the codelet
    /// is paused to stop the codelet completely instead of resuming stepping.
    fn resume(&mut self) -> Outcome {
        SKIPPED
    }
}

/// Context argument used for `Codelet` start, step and stop functions
pub struct Context<'a, C>
where
    C: Codelet + ?Sized,
{
    /// The instance clock provides timings of the default clock specific to this instance.
    pub clock: &'a TaskClock,

    /// The configuration used for this instance
    pub config: &'a C::Config,
}

/// All codelets can be converted into a CodeletInstance
///
/// ```
/// struct MyCodelet { num: u32 };
///
/// let c = MyCodelet{ num: 42 }.into_instance("my_name", MyCodelet::Config);
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
/// #[derive(Default)]
/// struct MyCodelet { flag: bool };
///
/// let c = MyCodelet::instantiate("my_name", MyCodelet::Config);
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
