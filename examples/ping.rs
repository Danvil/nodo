use core::time::Duration;
use nodo::codelet::ScheduleBuilder;
use nodo::prelude::*;
use nodo::runtime::Runtime;
use nodo_std::Sink;
use nodo_std::Source;

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
            .finalize(),
    );

    rt.wait_for_ctrl_c();

    Ok(())
}
