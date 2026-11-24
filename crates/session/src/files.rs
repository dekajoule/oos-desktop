// SPDX-License-Identifier: GPL-2.0-or-later
//! What the worker writes to files the user chose: the screen as PNG, the records of channels as CSV (time in
//! seconds, then volts per channel, from the preamble as OOS docs/scpi.md states it).

use oos_scpi::Waveform;

/// The records as CSV: a header, then one line per point. The records must share their time axis (the screen
/// records of one acquisition do); Err names the first that does not.
pub fn csv(waves: &[Waveform]) -> Result<String, String> {
    let Some(first) = waves.first() else { return Err("no record to save".into()) };
    for w in waves {
        let same = w.codes.len() == first.codes.len()
            && w.preamble.xincrement == first.preamble.xincrement
            && w.preamble.xorigin == first.preamble.xorigin;
        if !same {
            return Err(format!("CH{} does not share the time axis of CH{}", w.source, first.source));
        }
    }
    let mut out = String::from("time_s");
    for w in waves {
        out.push_str(&format!(",ch{}_V", w.source));
    }
    out.push('\n');
    for k in 0..first.codes.len() {
        out.push_str(&format!("{:e}", first.time(k)));
        for w in waves {
            out.push_str(&format!(",{:e}", w.volts(w.codes[k])));
        }
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oos_scpi::Preamble;

    fn wave(ch: u8, codes: Vec<u16>, xinc: f64) -> Waveform {
        let preamble = Preamble {
            format: 0,
            mode: 0,
            points: codes.len() as u64,
            xincrement: xinc,
            xorigin: -1e-3,
            xreference: 0.0,
            yincrement: 0.01,
            yorigin: 0.0,
            yreference: 128.0,
        };
        Waveform { source: ch, preamble, codes }
    }

    #[test]
    fn records_side_by_side() {
        let got = csv(&[wave(1, vec![128, 228], 1e-3), wave(3, vec![28, 128], 1e-3)]).expect("csv");
        assert_eq!(got, "time_s,ch1_V,ch3_V\n-1e-3,0e0,-1e0\n0e0,1e0,0e0\n");
    }

    #[test]
    fn records_of_other_time_axes_are_refused() {
        assert!(csv(&[wave(1, vec![1, 2], 1e-3), wave(2, vec![1, 2], 2e-3)]).is_err());
        assert!(csv(&[wave(1, vec![1, 2], 1e-3), wave(2, vec![1], 1e-3)]).is_err());
        assert!(csv(&[]).is_err());
    }
}
