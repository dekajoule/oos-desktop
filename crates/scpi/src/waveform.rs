// SPDX-License-Identifier: GPL-2.0-or-later
//! A record as `:WAVeform:PREamble?` and `:WAVeform:DATA?` give it (OOS docs/scpi.md, Waveform transfer), and its
//! conversion to seconds and volts, in one place.

/// The ten fields of `:WAVeform:PREamble?`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preamble {
    /// 0 BYTE, 1 WORD, 2 ASCII.
    pub format: u8,
    /// 0 NORM, 1 MAX, 2 RAW.
    pub mode: u8,
    pub points: u64,
    pub xincrement: f64,
    pub xorigin: f64,
    pub xreference: f64,
    pub yincrement: f64,
    pub yorigin: f64,
    pub yreference: f64,
}

impl Preamble {
    pub fn parse(s: &str) -> Option<Preamble> {
        let f: Vec<&str> = s.trim().split(',').map(str::trim).collect();
        if f.len() != 10 {
            return None;
        }
        let num = |i: usize| f[i].parse::<f64>().ok().filter(|x| x.is_finite());
        let int = |i: usize| f[i].parse::<u64>().ok();
        Some(Preamble {
            format: u8::try_from(int(0)?).ok()?,
            mode: u8::try_from(int(1)?).ok()?,
            points: int(2)?,
            xincrement: num(4)?,
            xorigin: num(5)?,
            xreference: num(6)?,
            yincrement: num(7)?,
            yorigin: num(8)?,
            yreference: num(9)?,
        })
    }

    /// Bytes per sample of the transfer format.
    pub fn width(&self) -> usize {
        if self.format == 1 {
            2
        } else {
            1
        }
    }
}

/// A record: its preamble and its codes.
#[derive(Clone, Debug, PartialEq)]
pub struct Waveform {
    pub source: u8,
    pub preamble: Preamble,
    pub codes: Vec<u16>,
}

impl Waveform {
    /// The codes of a block in the preamble's format (WORD is little-endian in OOS).
    pub fn codes_of(p: &Preamble, block: &[u8]) -> Vec<u16> {
        if p.width() == 2 {
            block.as_chunks::<2>().0.iter().map(|&b| u16::from_le_bytes(b)).collect()
        } else {
            block.iter().map(|&b| b as u16).collect()
        }
    }

    /// The time of sample `k` in seconds.
    pub fn time(&self, k: usize) -> f64 {
        let p = &self.preamble;
        (k as f64 - p.xreference) * p.xincrement + p.xorigin
    }

    /// The voltage of a code.
    pub fn volts(&self, code: u16) -> f64 {
        let p = &self.preamble;
        (code as f64 - p.yorigin - p.yreference) * p.yincrement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preamble_and_conversion() {
        let p = Preamble::parse("0,2,1000,1,1.0e-9,-5.0e-7,0,0.01,0,128").expect("preamble");
        assert_eq!((p.format, p.mode, p.points, p.width()), (0, 2, 1000, 1));
        let w = Waveform { source: 1, preamble: p, codes: Waveform::codes_of(&p, &[128, 228]) };
        assert_eq!(w.codes, vec![128, 228]);
        assert!((w.volts(228) - 1.0).abs() < 1e-12);
        assert!((w.time(500) - 0.0).abs() < 1e-15);
        let word = Preamble { format: 1, ..p };
        assert_eq!(Waveform::codes_of(&word, &[0x34, 0x12, 0xff]), vec![0x1234]);
    }

    #[test]
    fn broken_preambles_are_none() {
        for bad in ["", "0,2,1000", "0,2,x,1,1,0,0,1,0,128", "0,2,10,1,inf,0,0,1,0,128", "300,0,1,1,1,0,0,1,0,0"] {
            assert_eq!(Preamble::parse(bad), None, "{bad}");
        }
    }
}
