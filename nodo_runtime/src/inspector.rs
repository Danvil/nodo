use eyre::Result;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use nng::{
    options::{protocol::pubsub::Subscribe, Options},
    Protocol, Socket,
};
use nodo::{
    codelet::{NodeletId, Statistics},
    prelude::DefaultStatus,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Instant};

#[derive(Clone, Serialize, Deserialize)]
pub struct RenderedStatus {
    pub label: String,
    pub status: DefaultStatus,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct InspectorReport(HashMap<NodeletId, InspectorCodeletReport>);

impl InspectorReport {
    pub fn push(&mut self, id: NodeletId, entry: InspectorCodeletReport) {
        if self.0.contains_key(&id) {
            log::error!(
                "Duplicated codelet id: {:?} (name='{}', other='{}'). This will be a hard error in the future.",
                id,
                entry.name,
                self.0[&id].name
            );
        }
        self.0.insert(id, entry);
    }

    pub fn extend(&mut self, other: InspectorReport) {
        for (id, entry) in other.0 {
            self.push(id, entry);
        }
    }

    pub fn into_vec(self) -> Vec<(NodeletId, InspectorCodeletReport)> {
        self.0.into_iter().collect()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InspectorCodeletReport {
    pub sequence: String,
    pub name: String,
    pub typename: String,
    pub status: Option<RenderedStatus>,
    pub statistics: Statistics,
}

/// The server is running in the nodo runtime and publishes reports
pub struct InspectorServer {
    socket: Socket,
}

impl InspectorServer {
    pub fn open(address: &str) -> Result<Self> {
        log::info!("Opening Inspector PUB socket at '{}'..", address);

        let socket = Socket::new(Protocol::Pub0)?;

        socket.pipe_notify(move |_, ev| {
            log::trace!("pipe_notify: {ev:?}");
        })?;

        socket.listen(address)?;

        Ok(Self { socket })
    }

    pub fn send_report(&self, report: InspectorReport) -> Result<()> {
        let buffer = bincode::serialize(&report)?;
        let compressed = compress_prepend_size(&buffer);
        self.socket.send(&compressed).map_err(|(_, err)| err)?;
        Ok(())
    }
}

/// The client is running in the report viewer and receives reports
pub struct InspectorClient {
    socket: Socket,
    datarate: DatarateEstimation,
    last_report_time: Option<Instant>,
}

impl InspectorClient {
    pub fn dial(address: &str) -> Result<Self> {
        log::info!("Opening Inspector SUB socket at '{}'..", address);

        let socket = Socket::new(Protocol::Sub0)?;

        socket.pipe_notify(move |_, ev| {
            log::trace!("pipe_notify: {ev:?}");
        })?;

        socket.dial_async(address)?;

        // subscribe to all topics
        socket.set_opt::<Subscribe>(vec![])?;

        Ok(Self {
            socket,
            datarate: DatarateEstimation::default(),
            last_report_time: None,
        })
    }

    pub fn try_recv_report(&mut self) -> Result<Option<InspectorReport>> {
        let mut maybe_buff = None;
        loop {
            match self.socket.try_recv() {
                Ok(buff) => {
                    self.datarate.push(buff.len() as u64);
                    maybe_buff = Some(buff);
                }
                Err(nng::Error::TryAgain) => break,
                Err(err) => return Err(err)?,
            }
        }

        if let Some(buff) = maybe_buff {
            self.last_report_time = Some(Instant::now());
            let uncompressed = decompress_size_prepended(&buff)?;
            Ok(Some(bincode::deserialize(&uncompressed)?))
        } else {
            Ok(None)
        }
    }

    pub fn datarate(&self) -> f64 {
        self.datarate.datarate()
    }

    pub fn last_report_time(&self) -> Option<Instant> {
        self.last_report_time
    }
}

#[derive(Default)]
pub struct DatarateEstimation {
    total_bytes_received: u64,
    datarate: f64,
    last_step: Option<Instant>,
    bytes_since_last_step: u64,
}

impl DatarateEstimation {
    pub fn push(&mut self, len: u64) {
        self.bytes_since_last_step += len;
        self.total_bytes_received += len;

        let now = Instant::now();
        if let Some(prev) = self.last_step {
            let dt = (now - prev).as_secs_f64();
            if dt > 3.0 {
                self.last_step = Some(now);
                self.datarate =
                    0.2 * self.datarate + 0.8 * (self.bytes_since_last_step as f64) / dt;
                self.bytes_since_last_step = 0;
            }
        } else {
            self.last_step = Some(now);
        }
    }

    /// Datarate in bytes/s
    pub fn datarate(&self) -> f64 {
        self.datarate
    }
}
