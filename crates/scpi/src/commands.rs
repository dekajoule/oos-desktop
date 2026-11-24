// SPDX-License-Identifier: GPL-2.0-or-later
//! The table of settings (ADR 0004): one row per setting the program reads or sets, with its stable key, its SCPI
//! header (`{n}`: the channel), its kind and the label a panel shows. A new setting is a new row here and its key in a
//! panel's list; the headers are those of OOS docs/scpi.md at `OOS_REV`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A number with its unit and the range a control offers (the scope may take less and says so).
    Float {
        unit: &'static str,
        min: f64,
        max: f64,
    },
    Bool,
    /// One of these SCPI words, written with their short form in upper case (`NORMal`).
    Enum(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    /// Stable: stored in saved layouts, never changed once released.
    pub key: &'static str,
    pub header: &'static str,
    pub kind: Kind,
    pub label: &'static str,
}

impl Row {
    /// True when the header names a channel (`{n}`).
    pub fn per_channel(&self) -> bool {
        self.header.contains("{n}")
    }

    /// The header for channel `ch` (1-based); rows without a channel ignore it.
    pub fn header_for(&self, ch: u8) -> String {
        self.header.replace("{n}", &ch.to_string())
    }
}

/// The row of `key`.
pub fn row(key: &str) -> Option<&'static Row> {
    ROWS.iter().find(|r| r.key == key)
}

/// Words with a number in their short form are written as the short form (`CHAN1`): the short form of a word is the
/// part before its first lower-case letter, and the number would be lost.
const CHANNELS: &[&str] = &["CHAN1", "CHAN2", "CHAN3", "CHAN4"];

pub static ROWS: &[Row] = &[
    Row { key: "chan.on", header: ":CHANnel{n}:DISPlay", kind: Kind::Bool, label: "On" },
    Row {
        key: "chan.scale",
        header: ":CHANnel{n}:SCALe",
        kind: Kind::Float { unit: "V/div", min: 1e-3, max: 10.0 },
        label: "Scale",
    },
    Row {
        key: "chan.offset",
        header: ":CHANnel{n}:OFFSet",
        kind: Kind::Float { unit: "V", min: -550.0, max: 550.0 },
        label: "Offset",
    },
    Row {
        key: "chan.coupling",
        header: ":CHANnel{n}:COUPling",
        kind: Kind::Enum(&["DC", "AC", "GND"]),
        label: "Coupling",
    },
    Row {
        key: "chan.probe",
        header: ":CHANnel{n}:PROBe",
        kind: Kind::Float { unit: "×", min: 1e-4, max: 5e4 },
        label: "Probe",
    },
    Row {
        key: "chan.bandwidth",
        header: ":CHANnel{n}:BWLimit",
        kind: Kind::Enum(&["OFF", "20M", "AUTO"]),
        label: "BW limit",
    },
    Row { key: "chan.invert", header: ":CHANnel{n}:INVert", kind: Kind::Bool, label: "Invert" },
    Row {
        key: "time.scale",
        header: ":TIMebase:SCALe",
        kind: Kind::Float { unit: "s/div", min: 1e-9, max: 1000.0 },
        label: "Scale",
    },
    Row {
        key: "time.offset",
        header: ":TIMebase:OFFSet",
        kind: Kind::Float { unit: "s", min: -1000.0, max: 1000.0 },
        label: "Delay",
    },
    Row {
        key: "time.expand",
        header: ":TIMebase:HREFerence:MODE",
        kind: Kind::Enum(&["CENTer", "LB", "RB", "TRIGger"]),
        label: "Expand around",
    },
    Row { key: "time.roll", header: ":TIMebase:ROLL", kind: Kind::Bool, label: "Auto roll" },
    Row {
        key: "trig.sweep",
        header: ":TRIGger:SWEep",
        kind: Kind::Enum(&["AUTO", "NORMal", "SINGle"]),
        label: "Sweep",
    },
    Row { key: "trig.source", header: ":TRIGger:EDGE:SOURce", kind: Kind::Enum(CHANNELS), label: "Source" },
    Row {
        key: "trig.level",
        header: ":TRIGger:EDGE:LEVel",
        kind: Kind::Float { unit: "V", min: -1000.0, max: 1000.0 },
        label: "Level",
    },
    Row {
        key: "trig.slope",
        header: ":TRIGger:EDGE:SLOPe",
        kind: Kind::Enum(&["POSitive", "NEGative", "RFALl"]),
        label: "Slope",
    },
    Row {
        key: "trig.coupling",
        header: ":TRIGger:COUPling",
        kind: Kind::Enum(&["DC", "AC", "LFReject", "HFReject"]),
        label: "Coupling",
    },
    Row {
        key: "trig.holdoff",
        header: ":TRIGger:HOLDoff",
        kind: Kind::Float { unit: "s", min: 0.0, max: 10.0 },
        label: "Holdoff",
    },
    Row { key: "trig.noise", header: ":TRIGger:NREJect", kind: Kind::Bool, label: "Noise reject" },
    Row {
        key: "acq.type",
        header: ":ACQuire:TYPE",
        kind: Kind::Enum(&["NORMal", "PEAK", "AVERages", "HRESolution"]),
        label: "Mode",
    },
    Row {
        key: "acq.averages",
        header: ":ACQuire:AVERages",
        kind: Kind::Enum(&["2", "4", "8", "16", "32", "64", "128", "256", "512", "1024"]),
        label: "Averages",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::short;

    // Break it catches: a row without a header or label, a key used twice or in another form than `part.name`, a
    // range the wrong way round, an enum word without a short form.
    #[test]
    fn every_row_is_complete() {
        let mut keys = std::collections::HashSet::new();
        for r in ROWS {
            assert!(keys.insert(r.key), "{}: key used twice", r.key);
            let ok_key = r.key.split('.').count() == 2 && r.key.bytes().all(|c| c.is_ascii_lowercase() || c == b'.');
            assert!(ok_key, "{}: keys are `part.name` in lower case", r.key);
            assert!(r.header.starts_with(':') && !r.header.ends_with('?'), "{}: header {}", r.key, r.header);
            assert!(!r.label.is_empty(), "{}: no label", r.key);
            match r.kind {
                Kind::Float { unit, min, max } => assert!(min < max && !unit.is_empty(), "{}: range or unit", r.key),
                Kind::Enum(words) => {
                    assert!(!words.is_empty(), "{}: no words", r.key);
                    for w in words {
                        let s = short(w);
                        assert!(!s.is_empty(), "{}: {w} has no short form", r.key);
                        let rest_lower = w[s.len()..].bytes().all(|c| c.is_ascii_lowercase());
                        assert!(rest_lower, "{}: {w}: after the short form only lower case (write `CHAN1`)", r.key);
                    }
                }
                Kind::Bool => {}
            }
        }
    }
}
