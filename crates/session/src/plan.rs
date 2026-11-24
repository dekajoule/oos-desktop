// SPDX-License-Identifier: GPL-2.0-or-later
//! The poll plan: what is read how often, only what a visible panel needs. The waveforms of the watched channels as
//! fast as the link delivers them, one after the other; the settings once a second. Pure, with the time handed in.

use std::time::{Duration, Instant};

/// How often all settings are read again.
pub const SETTINGS_EVERY: Duration = Duration::from_secs(1);

/// What the visible panels need.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Watch {
    /// Channels whose waveform is shown (1-based).
    pub channels: Vec<u8>,
    /// Whether a panel with settings is open.
    pub settings: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Job {
    Settings,
    Waveform(u8),
}

pub struct Plan {
    pub watch: Watch,
    settings_at: Option<Instant>,
    next_wave: usize,
}

impl Plan {
    pub fn new() -> Plan {
        Plan { watch: Watch::default(), settings_at: None, next_wave: 0 }
    }

    /// The next job at `now`, or None when nothing is due.
    pub fn next(&mut self, now: Instant) -> Option<Job> {
        let due = self.settings_at.is_none_or(|t| now.duration_since(t) >= SETTINGS_EVERY);
        if self.watch.settings && due {
            self.settings_at = Some(now);
            return Some(Job::Settings);
        }
        let ch = self.watch.channels.get(self.next_wave % self.watch.channels.len().max(1)).copied()?;
        self.next_wave = self.next_wave.wrapping_add(1);
        Some(Job::Waveform(ch))
    }

    /// Settings are read again at the next job (after a set, a run control or a new watch).
    pub fn settings_now(&mut self) {
        self.settings_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_once_a_second_waveforms_in_turn() {
        let t0 = Instant::now();
        let mut p = Plan::new();
        assert_eq!(p.next(t0), None);
        p.watch = Watch { channels: vec![1, 3], settings: true };
        let jobs: Vec<_> = (0..4).map(|i| p.next(t0 + Duration::from_millis(i * 10))).collect();
        assert_eq!(
            jobs,
            vec![Some(Job::Settings), Some(Job::Waveform(1)), Some(Job::Waveform(3)), Some(Job::Waveform(1))]
        );
        assert_eq!(p.next(t0 + SETTINGS_EVERY), Some(Job::Settings));
        p.settings_now();
        assert_eq!(p.next(t0 + SETTINGS_EVERY), Some(Job::Settings));
    }
}
