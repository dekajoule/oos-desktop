// SPDX-License-Identifier: GPL-2.0-or-later
//! The worker thread: the actions first, merged, then the poll plan. A lost link ends it with [`Event::Lost`]; there
//! is no silent reconnect.

use crate::merge::merge;
use crate::plan::{Job, Plan};
use crate::{Action, Event};
use oos_scpi::{commands, measures, Scope, ScpiError, ROWS};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// How long opening a link may take.
pub const CONNECT_WAIT: Duration = Duration::from_secs(5);
/// How long the worker sleeps when nothing is due.
const IDLE: Duration = Duration::from_millis(50);

pub fn run(
    connect: impl FnOnce() -> Result<Box<dyn oos_link::Link>, oos_link::Error>,
    actions: Receiver<Action>,
    events: Sender<Event>,
    wake: impl Fn(),
) {
    let emit = |e: Event| {
        let ok = events.send(e).is_ok();
        wake();
        ok
    };
    let mut scope = match connect() {
        Ok(l) => Scope::new(l),
        Err(e) => {
            emit(Event::Lost(e.to_string()));
            return;
        }
    };
    let id = match scope.identity() {
        Ok(id) => id,
        Err(e) => {
            emit(Event::Lost(e.to_string()));
            return;
        }
    };
    let channels = id.channels();
    if !emit(Event::Connected(id)) {
        return;
    }
    let mut plan = Plan::new();
    loop {
        // the actions first: the newest of each setting, in their order
        let mut waiting: Vec<Action> = actions.try_iter().collect();
        if waiting.is_empty() && plan.watch == Default::default() {
            match actions.recv_timeout(IDLE) {
                Ok(a) => waiting.push(a),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return,
            }
            waiting.extend(actions.try_iter());
        }
        for a in merge(waiting) {
            let r = match a {
                Action::Close => return,
                Action::Watch(w) => {
                    plan.watch = w;
                    plan.settings_now();
                    Ok(())
                }
                Action::Set { key, ch, value } => match commands::row(key) {
                    Some(row) => {
                        let r = scope.set(row, ch, &value);
                        plan.settings_now();
                        r.map_err(|e| (Some(key), e))
                    }
                    None => {
                        Err((Some(key), ScpiError::Unexpected { asked: key.into(), got: "no such setting".into() }))
                    }
                },
                Action::Run | Action::Stop | Action::Single => {
                    plan.settings_now();
                    match a {
                        Action::Run => scope.run(),
                        Action::Stop => scope.stop(),
                        _ => scope.single(),
                    }
                    .map_err(|e| (None, e))
                }
            };
            if let Err((key, e)) = r {
                if !report(&emit, key, e) {
                    return;
                }
            }
        }
        // then what the panels watch
        let r = match plan.next(Instant::now()) {
            None => continue,
            Some(Job::Settings) => read_settings(&mut scope, channels, &emit),
            Some(Job::Measures) => read_measures(&mut scope, &plan.watch.measures, &emit),
            Some(Job::Waveform(ch)) => scope.waveform(ch).map(|w| {
                emit(Event::Waveform(w));
            }),
        };
        if let Err(e) = r {
            if !report(&emit, None, e) {
                return;
            }
        }
    }
}

/// Every row, for each channel of the model.
fn read_settings(scope: &mut Scope, channels: u8, emit: &impl Fn(Event) -> bool) -> Result<(), ScpiError> {
    for row in ROWS {
        let chs = if row.per_channel() { 1..=channels } else { 1..=1 };
        for ch in chs {
            let value = scope.get(row, ch)?;
            emit(Event::Setting { key: row.key, ch, value });
        }
    }
    Ok(())
}

/// The watched measurements; a key without a row is skipped (the app names only rows of `MEASURES`).
fn read_measures(
    scope: &mut Scope,
    which: &[(&'static str, u8)],
    emit: &impl Fn(Event) -> bool,
) -> Result<(), ScpiError> {
    for &(key, ch) in which {
        if let Some(m) = measures::measure(key) {
            let value = scope.measure(m, ch)?;
            emit(Event::Measured { key, ch, value });
        }
    }
    Ok(())
}

/// A refusal or an odd answer is a problem the session survives; a link error ends it. False: end the worker.
fn report(emit: &impl Fn(Event) -> bool, key: Option<&'static str>, e: ScpiError) -> bool {
    match e {
        ScpiError::Link(e) => {
            emit(Event::Lost(e.to_string()));
            false
        }
        e => emit(Event::Problem { key, text: e.to_string() }),
    }
}
