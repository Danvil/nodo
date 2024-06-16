// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::marker::PhantomData;
use nodo::prelude::*;

/// A codelet which converts messages using the Into trait.
pub struct Convert<T, S> {
    marker: PhantomData<(T, S)>,
}

impl<T, S> Default for Convert<T, S> {
    fn default() -> Self {
        Convert {
            marker: PhantomData,
        }
    }
}

impl<T, S> Codelet for Convert<T, S>
where
    T: Send + Sync,
    S: Clone + Send + Sync + From<T>,
{
    type Config = ();
    type Rx = DoubleBufferRx<T>;
    type Tx = DoubleBufferTx<S>;

    fn build_bundles(_: &Self::Config) -> (Self::Rx, Self::Tx) {
        (
            DoubleBufferRx::new_auto_size(),
            DoubleBufferTx::new_auto_size(),
        )
    }

    fn step(&mut self, _: &Context<Self>, rx: &mut Self::Rx, tx: &mut Self::Tx) -> Outcome {
        while let Some(msg) = rx.try_pop() {
            tx.push(msg.into())?;
        }
        SUCCESS
    }
}

#[cfg(test)]
mod tests {
    // FIXME This test currently does not terminate. Termination needs to be properly implemented.

    // use crate::Convert;
    // use crate::Sink;
    // use crate::Source;
    // use nodo::codelet::ScheduleBuilder;
    // use nodo::prelude::*;
    // use nodo::runtime::Runtime;

    // #[test]
    // fn test_convert() {
    //     #[derive(Debug, Clone)]
    //     struct Foo {
    //         number: u32,
    //     }

    //     #[derive(Debug, Clone)]
    //     struct Bar {
    //         text: String,
    //     }

    //     impl From<Foo> for Bar {
    //         fn from(foo: Foo) -> Bar {
    //             Bar {
    //                 text: format!("{}", foo.number),
    //             }
    //         }
    //     }

    //     let mut rt = Runtime::new();

    //     let mut source = Source::new(|| Foo { number: 42 }).into_instance("source", ());

    //     let mut sink = Sink::new(|bar: Bar| {
    //         assert_eq!(bar.text, "42");
    //         SUCCESS
    //     })
    //     .into_instance("sink", ());

    //     let mut into = Convert::instantiate("into", ());

    //     source.tx.connect(&mut into.rx).unwrap();
    //     into.tx.connect(&mut sink.rx).unwrap();

    //     rt.add_codelet_schedule(
    //         ScheduleBuilder::new()
    //             .with_max_step_count(10)
    //             .with(source)
    //             .with(into)
    //             .with(sink)
    //             .finalize(),
    //     );

    //     rt.spin();
    // }
}
