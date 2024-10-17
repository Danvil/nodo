use core::time::Duration;
use nodo::{codelet::ScheduleBuilder, prelude::*};
use nodo_runtime::Runtime;
use nodo_std::{Sink, Source};

#[derive(Debug, Clone)]
struct Ping;

fn main() -> eyre::Result<()> {
    let mut rt = Runtime::new();

    let mut source = Source::new(|| Ping).into_instance("source", ());

    let mut sink = Sink::new(|x| {
        println!("{x:?}");
        SUCCESS
    })
    .into_instance("sink", ());

    source.tx.connect(&mut sink.rx)?;

    rt.add_codelet_schedule(
        ScheduleBuilder::new()
            .with_period(Duration::from_millis(100))
            .with(source)
            .with(sink)
            .into(),
    );

    rt.enable_terminate_on_ctrl_c();
    rt.spin();

    Ok(())
}
