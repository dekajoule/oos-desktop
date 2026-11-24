// SPDX-License-Identifier: GPL-2.0-or-later
//! What the window knows of the scope: the last answer of each setting, the values sent and not yet confirmed, the
//! last record of each channel, the last measurements, the identity and the last problem. Changed only by events and
//! by what the UI sends; `measures_shown` is the window's own choice of measurements, kept across connections.

use oos_session::{Event, Identity, Value, Waveform};
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub struct Model {
    pub identity: Option<Identity>,
    settings: HashMap<(&'static str, u8), Value>,
    pending: HashMap<(&'static str, u8), Value>,
    pub waves: BTreeMap<u8, Waveform>,
    measured: HashMap<(&'static str, u8), Option<f64>>,
    pub problem: Option<String>,
    /// The last thing done that the user should see (a file written).
    pub notice: Option<String>,
    pub lost: Option<String>,
}

impl Model {
    pub fn apply(&mut self, e: Event) {
        match e {
            Event::Connected(id) => {
                *self = Model { identity: Some(id), ..Model::default() };
            }
            Event::Setting { key, ch, value } => {
                if self.pending.get(&(key, ch)) == Some(&value) {
                    self.pending.remove(&(key, ch));
                }
                self.settings.insert((key, ch), value);
            }
            Event::Waveform(w) => {
                self.waves.insert(w.source, w);
            }
            Event::Measured { key, ch, value } => {
                self.measured.insert((key, ch), value);
            }
            Event::Problem { key, text } => {
                if let Some(k) = key {
                    self.pending.retain(|(pk, _), _| *pk != k);
                }
                self.problem = Some(text);
            }
            Event::Saved(path) => {
                self.notice = Some(format!("saved {}", path.display()));
            }
            Event::Lost(why) => {
                self.lost = Some(why);
            }
        }
    }

    /// A value sent; shown as pending until the scope answers it.
    pub fn sent(&mut self, key: &'static str, ch: u8, v: Value) {
        self.pending.insert((key, ch), v);
    }

    /// What a control shows: the value being sent, else the scope's, and whether it is still pending.
    pub fn value(&self, key: &'static str, ch: u8) -> Option<(&Value, bool)> {
        self.pending.get(&(key, ch)).map(|v| (v, true)).or_else(|| self.settings.get(&(key, ch)).map(|v| (v, false)))
    }

    /// The last value of a measurement: None before it was read, Some(None) while the scope has no value.
    pub fn measured(&self, key: &'static str, ch: u8) -> Option<Option<f64>> {
        self.measured.get(&(key, ch)).copied()
    }

    /// The channels switched on, as the scope last answered.
    pub fn channels_on(&self) -> Vec<u8> {
        (1..=self.channels()).filter(|&c| matches!(self.value("chan.on", c), Some((Value::Bool(true), _)))).collect()
    }

    pub fn channels(&self) -> u8 {
        self.identity.as_ref().map_or(4, Identity::channels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sent_value_is_pending_until_the_scope_answers_it() {
        let mut m = Model::default();
        m.sent("chan.scale", 1, Value::Float(0.5));
        assert_eq!(m.value("chan.scale", 1), Some((&Value::Float(0.5), true)));
        m.apply(Event::Setting { key: "chan.scale", ch: 1, value: Value::Float(0.2) });
        assert_eq!(m.value("chan.scale", 1), Some((&Value::Float(0.5), true)), "an older answer");
        m.apply(Event::Setting { key: "chan.scale", ch: 1, value: Value::Float(0.5) });
        assert_eq!(m.value("chan.scale", 1), Some((&Value::Float(0.5), false)));
    }

    #[test]
    fn a_refused_value_is_no_longer_pending() {
        let mut m = Model::default();
        m.apply(Event::Setting { key: "time.scale", ch: 1, value: Value::Float(1e-3) });
        m.sent("time.scale", 1, Value::Float(1e9));
        m.apply(Event::Problem { key: Some("time.scale"), text: "refused".into() });
        assert_eq!(m.value("time.scale", 1), Some((&Value::Float(1e-3), false)));
        assert_eq!(m.problem.as_deref(), Some("refused"));
    }
}
