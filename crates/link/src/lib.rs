// SPDX-License-Identifier: GPL-2.0-or-later
//! Bytes to and from one scope (docs/architecture.md 3.1). A [`Link`] sends a command and returns each answer as one
//! complete [`Message`]: a line, or an IEEE 488.2 definite-length block. It knows nothing of SCPI beyond that framing,
//! and it does not retry. Two transports: the SCPI socket ([`Tcp`]) and USBTMC ([`Usbtmc`]).

mod discover;
mod endpoint;
mod framing;
mod script;
mod tcp;
mod usbtmc;

pub use discover::{discover, Found};
pub use endpoint::{Endpoint, SCPI_PORT};
pub use framing::{Framer, MAX_BLOCK, MAX_LINE};
pub use script::Script;
pub use tcp::Tcp;
pub use usbtmc::Usbtmc;

use std::time::Instant;

/// One complete answer of the scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// A text answer without its line end.
    Line(String),
    /// The payload of a definite-length block, without its header and the newline after it.
    Block(Vec<u8>),
}

/// What can go wrong between the program and the scope.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// The deadline passed before a whole answer came.
    Timeout,
    /// The scope closed the connection.
    Closed,
    /// The bytes do not frame as an answer; `got` is the start of them, for the message.
    Malformed {
        what: &'static str,
        got: Vec<u8>,
    },
    /// The endpoint is known but this build cannot open it yet.
    NotSupported(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Timeout => write!(f, "no answer in time"),
            Error::Closed => write!(f, "the scope closed the connection"),
            Error::Malformed { what, got } => write!(f, "{what}: {:?}", String::from_utf8_lossy(got)),
            Error::NotSupported(what) => write!(f, "not supported yet: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Error::Timeout,
            std::io::ErrorKind::UnexpectedEof => Error::Closed,
            _ => Error::Io(e),
        }
    }
}

/// A connection to one scope. Commands and queries are sent as text without the line end; the link adds what its
/// transport needs.
pub trait Link: Send {
    /// Sends one command or query.
    fn send(&mut self, cmd: &str) -> Result<(), Error>;
    /// The next whole answer, or [`Error::Timeout`] at `deadline`.
    fn recv(&mut self, deadline: Instant) -> Result<Message, Error>;
    /// Drops what is buffered and in flight (a device clear where the transport has one).
    fn clear(&mut self) -> Result<(), Error>;
    /// Where this link goes.
    fn endpoint(&self) -> Endpoint;
}

/// Opens a link to `ep`.
pub fn open(ep: &Endpoint, deadline: Instant) -> Result<Box<dyn Link>, Error> {
    match ep {
        Endpoint::Tcp { .. } => Ok(Box::new(Tcp::connect(ep.clone(), deadline)?)),
        Endpoint::Usb { .. } => Ok(Box::new(Usbtmc::open(ep.clone())?)),
    }
}
