// Copyright 2022 by David Weikersdorfer

use crate::nodo::inspector as nodi;
use nng::options::protocol::pubsub::Subscribe;
use nng::options::Options;
use nng::*;
use prost::Message;

pub struct NodoAppLink {
    socket: nng::Socket,
    pub last_message_size: usize,
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    BrokenConnection,
    MalformedMessage,
}

#[derive(Debug, Clone)]
pub struct Error {
    _code: ErrorCode,
}

pub type Result<T> = std::result::Result<T, Error>;

impl NodoAppLink {
    pub fn open(address: &str) -> NodoAppLink {
        let socket = Socket::new(Protocol::Sub0).unwrap();

        socket
            .pipe_notify(move |_, ev| {
                log::trace!("nng::socket::pipe_notify: {ev:?}");
            })
            .unwrap();

        socket.dial_async(address).unwrap();

        socket.set_opt::<Subscribe>(vec![]).unwrap();

        NodoAppLink {
            socket,
            last_message_size: 0,
        }
    }

    pub fn request(&mut self) -> Result<nodi::Worldstate> {
        let buff = self.socket.recv().or(Err(Error {
            _code: ErrorCode::BrokenConnection,
        }))?;
        self.last_message_size = buff.len();
        nodi::Worldstate::decode(&buff[..]).or(Err(Error {
            _code: ErrorCode::MalformedMessage,
        }))
    }
}
