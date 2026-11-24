// SPDX-License-Identifier: GPL-2.0-or-later
//! The measurements the program shows, one row each: a stable key, the item of `:MEASure:ITEM?` (OOS docs/scpi.md,
//! Standard measurement items), the label and the unit. The scope measures (ADR 0002); a new measurement is a new
//! row here.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measure {
    /// Stable: stored with the window's settings, never changed once released.
    pub key: &'static str,
    /// The item as `:MEASure:ITEM?` takes it.
    pub item: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
}

pub static MEASURES: &[Measure] = &[
    Measure { key: "vpp", item: "VPP", label: "Peak-peak", unit: "V" },
    Measure { key: "vmax", item: "VMAX", label: "Maximum", unit: "V" },
    Measure { key: "vmin", item: "VMIN", label: "Minimum", unit: "V" },
    Measure { key: "vavg", item: "VAVG", label: "Mean", unit: "V" },
    Measure { key: "vrms", item: "VRMS", label: "RMS", unit: "V" },
    Measure { key: "freq", item: "FREQUENCY", label: "Frequency", unit: "Hz" },
    Measure { key: "period", item: "PERIOD", label: "Period", unit: "s" },
    Measure { key: "rise", item: "RTIME", label: "Rise time", unit: "s" },
    Measure { key: "fall", item: "FTIME", label: "Fall time", unit: "s" },
    Measure { key: "duty", item: "PDUTY", label: "+Duty", unit: "%" },
];

/// The row of `key`.
pub fn measure(key: &str) -> Option<&'static Measure> {
    MEASURES.iter().find(|m| m.key == key)
}

/// What OOS answers for a value it has not got (pending, or not measurable on this record).
pub const NO_VALUE: f64 = 9.9e37;

#[cfg(test)]
mod tests {
    use super::*;

    // Break it catches: a key used twice or not in lower case, an item not in the form OOS takes, a row without label
    // or unit
    #[test]
    fn every_measure_is_complete() {
        let mut keys = std::collections::HashSet::new();
        for m in MEASURES {
            assert!(keys.insert(m.key), "{}: key used twice", m.key);
            assert!(m.key.bytes().all(|c| c.is_ascii_lowercase()), "{}: keys are lower case", m.key);
            assert!(m.item.bytes().all(|c| c.is_ascii_uppercase()), "{}: item {}", m.key, m.item);
            assert!(!m.label.is_empty() && !m.unit.is_empty(), "{}: label and unit", m.key);
        }
    }
}
