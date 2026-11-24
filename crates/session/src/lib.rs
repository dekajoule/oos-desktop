// SPDX-License-Identifier: GPL-2.0-or-later
//! One connection to one scope, off the UI thread (docs/architecture.md 3.3, ADR 0005). [`open`] starts a worker that
//! owns the link; the UI sends [`Action`]s and takes [`Event`]s. The types the UI needs of the layers below are
//! re-exported here, so that `app` uses no crate below this one (ADR 0003).

mod files;
mod merge;
mod plan;
mod worker;

pub use oos_link::{Endpoint, Found};
pub use oos_scpi::{
    commands, measures, short, Identity, Kind, Measure, Preamble, Row, Value, Waveform, MEASURES, ROWS,
};
pub use plan::Watch;

use std::sync::mpsc::{channel, Receiver, Sender};

/// What the UI asks of the scope.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Set the setting `key` (a row of the table) of channel `ch` (1-based; ignored by rows without a channel).
    Set {
        key: &'static str,
        ch: u8,
        value: Value,
    },
    Run,
    Stop,
    Single,
    /// What the visible panels need read regularly.
    Watch(Watch),
    /// The scope's screen as PNG into `path` (a file the user chose).
    SaveScreenshot(std::path::PathBuf),
    /// The records of channels `chs` read again and written as CSV into `path`.
    SaveRecords {
        chs: Vec<u8>,
        path: std::path::PathBuf,
    },
    Close,
}

/// What the worker learned.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Connected(Identity),
    /// The value the scope answered for a setting.
    Setting {
        key: &'static str,
        ch: u8,
        value: Value,
    },
    Waveform(Waveform),
    /// A measurement of a channel; None while the scope has no value for it.
    Measured {
        key: &'static str,
        ch: u8,
        value: Option<f64>,
    },
    /// A file was written.
    Saved(std::path::PathBuf),
    /// The scope refused an action, or answered something that does not fit; the session goes on.
    Problem {
        key: Option<&'static str>,
        text: String,
    },
    /// The link is gone; the worker has ended.
    Lost(String),
}

/// The UI's end of a session.
pub struct Handle {
    tx: Sender<Action>,
}

impl Handle {
    /// Queues an action; false when the worker has ended.
    pub fn send(&self, a: Action) -> bool {
        self.tx.send(a).is_ok()
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = self.tx.send(Action::Close);
    }
}

/// Starts a session to `ep`. `wake` is called after each event, so the UI draws again (the app hands in egui's
/// repaint request; this crate does not know egui).
pub fn open(ep: Endpoint, wake: impl Fn() + Send + 'static) -> (Handle, Receiver<Event>) {
    start(move || oos_link::open(&ep, std::time::Instant::now() + worker::CONNECT_WAIT), wake)
}

/// Looks for scopes on the network in a thread of its own (an mDNS question, answers collected for about two
/// seconds); the list comes through the receiver, then `wake` is called.
pub fn find(wake: impl Fn() + Send + 'static) -> Receiver<Result<Vec<Found>, String>> {
    let (tx, rx) = channel();
    let spawned = std::thread::Builder::new().name("oos-find".into()).spawn(move || {
        let _ = tx.send(oos_link::discover(std::time::Duration::from_secs(2)).map_err(|e| e.to_string()));
        wake();
    });
    if let Err(e) = spawned {
        eprintln!("oos-session: no thread to find scopes: {e}");
    }
    rx
}

/// A session on a link made by `connect` in the worker: for tests with a scripted link.
pub fn start(
    connect: impl FnOnce() -> Result<Box<dyn oos_link::Link>, oos_link::Error> + Send + 'static,
    wake: impl Fn() + Send + 'static,
) -> (Handle, Receiver<Event>) {
    let (tx, actions) = channel();
    let (events, rx) = channel();
    std::thread::Builder::new()
        .name("oos-session".into())
        .spawn(move || worker::run(connect, actions, events, wake))
        .map(|_| ())
        .unwrap_or_else(|e| eprintln!("oos-session: no worker thread: {e}"));
    (Handle { tx }, rx)
}
