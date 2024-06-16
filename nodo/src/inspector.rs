use crate::codelet::WorkerReport;
use inspector_proto as insp;
use nng::Protocol;
use nng::Socket;
use prost::Message;
use std::time::Instant;

pub struct Inspector {
    start: Instant,
    socket: nng::Socket,
}

impl Inspector {
    pub fn open(address: &str) -> eyre::Result<Self> {
        let socket = Socket::new(Protocol::Pub0)?;

        socket.pipe_notify(move |_, ev| {
            log::trace!("nng::socket::pipe_notify: {ev:?}");
        })?;

        socket.listen(address)?;
        Ok(Self {
            start: Instant::now(),
            socket,
        })
    }

    pub fn send(&self, report: &WorkerReport) {
        println!("INSPECTOR SEND");
        println!("{report:?}");

        let mut state = insp::Worldstate::default();

        state.manifold = Some(insp::Manifold {
        	vertices: 
        });

        state.app_time = (Instant::now() - self.start).as_millis() as i64;
        state.system_time = state.app_time; // TODO

        let buf = state.encode_to_vec();
        match self.socket.send(&buf) {
            Err(err) => log::error!("{err:?}"),
            Ok(_) => {}
        }
    }
}
