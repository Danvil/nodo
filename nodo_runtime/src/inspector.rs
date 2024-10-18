use eyre::Result;
use nng::{
    options::{protocol::pubsub::Subscribe, Options},
    Protocol, Socket,
};
use nodo::{codelet::Statistics, prelude::DefaultStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct RenderedStatus {
    pub label: String,
    pub status: DefaultStatus,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct InspectorReport(HashMap<String, InspectorCodeletReport>);

impl InspectorReport {
    pub fn push(&mut self, entry: InspectorCodeletReport) {
        if self.0.contains_key(&entry.name) {
            log::error!(
                "Duplicated codelet name: {}. This will be a hard error in the future.",
                entry.name
            );
        }
        self.0.insert(entry.name.clone(), entry);
    }

    pub fn extend(&mut self, other: InspectorReport) {
        for (_, entry) in other.0 {
            self.push(entry);
        }
    }

    pub fn into_vec(self) -> Vec<InspectorCodeletReport> {
        self.0.into_iter().map(|(_, v)| v).collect()
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
        self.socket.send(&buffer).map_err(|(_, err)| err)?;
        Ok(())
    }
}

/// The client is running in the report viewer and receives reports
pub struct InspectorClient {
    socket: Socket,
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

        Ok(Self { socket })
    }

    pub fn try_recv_report(&self) -> Result<Option<InspectorReport>> {
        let mut maybe_buff = None;
        loop {
            match self.socket.try_recv() {
                Ok(buff) => {
                    println!("{}", buff.len());
                    maybe_buff = Some(buff);
                }
                Err(nng::Error::TryAgain) => break,
                Err(err) => return Err(err)?,
            }
        }
        Ok(maybe_buff
            .as_ref()
            .map(|buff| bincode::deserialize(buff))
            .transpose()?)
    }
}
