// SPDX-License-Identifier: GPL-2.0-or-later
//! The typed side of the scope: identity, the settings of the table, run control, records and the error queue.

use crate::commands::Row;
use crate::{Preamble, Value, Waveform};
use oos_link::{Error as LinkError, Link, Message};
use std::time::{Duration, Instant};

/// How long a line answer may take.
const LINE_WAIT: Duration = Duration::from_secs(2);
/// Points read per `:WAVeform:DATA?` page.
pub const PAGE: u64 = 1_000_000;
/// Error queue entries read after a set at most (a scope that keeps answering is not waited for forever).
const MAX_ERRORS: usize = 16;

#[derive(Debug)]
pub enum ScpiError {
    Link(LinkError),
    /// The scope refused a command: its error queue entries.
    Refused(Vec<(i32, String)>),
    /// An answer that does not fit what was asked.
    Unexpected {
        asked: String,
        got: String,
    },
}

impl std::fmt::Display for ScpiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScpiError::Link(e) => write!(f, "{e}"),
            ScpiError::Refused(v) => {
                let t: Vec<String> = v.iter().map(|(c, m)| format!("{c}, {m}")).collect();
                write!(f, "refused: {}", t.join("; "))
            }
            ScpiError::Unexpected { asked, got } => write!(f, "{asked} answered {got:?}"),
        }
    }
}

impl std::error::Error for ScpiError {}

impl From<LinkError> for ScpiError {
    fn from(e: LinkError) -> Self {
        ScpiError::Link(e)
    }
}

/// `*IDN?`: `OOS,<model>,<serial>,oos-ui <version>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub maker: String,
    pub model: String,
    pub serial: String,
    pub version: String,
}

impl Identity {
    /// The analog channels the model has: the last digit of `DHO804`, `DHO924S`, `MHO934`.
    pub fn channels(&self) -> u8 {
        let digits: String = self.model.chars().filter(|c| c.is_ascii_digit()).collect();
        match digits.chars().last().and_then(|c| c.to_digit(10)) {
            Some(n @ 1..=4) => n as u8,
            _ => 4,
        }
    }
}

pub struct Scope {
    link: Box<dyn Link>,
}

impl Scope {
    pub fn new(link: Box<dyn Link>) -> Scope {
        Scope { link }
    }

    pub fn identity(&mut self) -> Result<Identity, ScpiError> {
        let a = self.query("*IDN?")?;
        let f: Vec<&str> = a.splitn(4, ',').map(str::trim).collect();
        if f.len() != 4 {
            return Err(ScpiError::Unexpected { asked: "*IDN?".into(), got: a });
        }
        let version = f[3].strip_prefix("oos-ui ").unwrap_or(f[3]).to_string();
        Ok(Identity { maker: f[0].into(), model: f[1].into(), serial: f[2].into(), version })
    }

    /// The value of a setting (`ch`: 1-based, ignored by rows without a channel).
    pub fn get(&mut self, row: &Row, ch: u8) -> Result<Value, ScpiError> {
        let q = format!("{}?", row.header_for(ch));
        let a = self.query(&q)?;
        Value::parse(&row.kind, &a).ok_or(ScpiError::Unexpected { asked: q, got: a })
    }

    /// Sets a setting and reads the error queue, so a refused value is an error of this call.
    pub fn set(&mut self, row: &Row, ch: u8, v: &Value) -> Result<(), ScpiError> {
        self.command(&format!("{} {}", row.header_for(ch), v.to_scpi()))
    }

    pub fn run(&mut self) -> Result<(), ScpiError> {
        self.command(":RUN")
    }

    pub fn stop(&mut self) -> Result<(), ScpiError> {
        self.command(":STOP")
    }

    pub fn single(&mut self) -> Result<(), ScpiError> {
        self.command(":SINGle")
    }

    /// The screen record of channel `ch` (`:WAVeform:MODE NORMal`, BYTE), read in pages of [`PAGE`] points.
    pub fn waveform(&mut self, ch: u8) -> Result<Waveform, ScpiError> {
        self.command(&format!(":WAVeform:SOURce CHANnel{ch}"))?;
        self.command(":WAVeform:MODE NORMal")?;
        self.command(":WAVeform:FORMat BYTE")?;
        self.command(":WAVeform:STARt 1")?;
        let pre = self.query(":WAVeform:PREamble?")?;
        let p = Preamble::parse(&pre).ok_or(ScpiError::Unexpected { asked: ":WAVeform:PREamble?".into(), got: pre })?;
        let mut codes = Vec::with_capacity(p.points.min(PAGE * 4) as usize);
        let mut k = 1;
        while k <= p.points {
            let last = (k + PAGE - 1).min(p.points);
            self.link.send(&format!(":WAVeform:STARt {k};:WAVeform:STOP {last}"))?;
            let b = self.block(":WAVeform:DATA?", (last - k + 1) as usize * p.width())?;
            codes.extend(Waveform::codes_of(&p, &b));
            k = last + 1;
        }
        Ok(Waveform { source: ch, preamble: p, codes })
    }

    /// The error queue until it is empty (`0,"No error"`), at most [`MAX_ERRORS`] entries.
    pub fn errors(&mut self) -> Result<Vec<(i32, String)>, ScpiError> {
        let mut out = Vec::new();
        for _ in 0..MAX_ERRORS {
            let a = self.query(":SYSTem:ERRor?")?;
            let (code, text) = a.split_once(',').unwrap_or((a.as_str(), ""));
            let code: i32 = code
                .trim()
                .parse()
                .map_err(|_| ScpiError::Unexpected { asked: ":SYSTem:ERRor?".into(), got: a.clone() })?;
            if code == 0 {
                break;
            }
            out.push((code, text.trim().trim_matches('"').to_string()));
        }
        Ok(out)
    }

    /// Sends a command and fails with the error queue if it is not empty afterwards.
    fn command(&mut self, cmd: &str) -> Result<(), ScpiError> {
        self.link.send(cmd)?;
        let e = self.errors()?;
        if e.is_empty() {
            Ok(())
        } else {
            Err(ScpiError::Refused(e))
        }
    }

    fn query(&mut self, q: &str) -> Result<String, ScpiError> {
        self.link.send(q)?;
        match self.link.recv(Instant::now() + LINE_WAIT)? {
            Message::Line(s) => Ok(s),
            Message::Block(b) => Err(ScpiError::Unexpected { asked: q.into(), got: format!("a block of {}", b.len()) }),
        }
    }

    /// A block query; the wait grows with the bytes announced (10 MB/s at least, as over USB 2).
    fn block(&mut self, q: &str, expect: usize) -> Result<Vec<u8>, ScpiError> {
        self.link.send(q)?;
        let wait = LINE_WAIT + Duration::from_millis(expect as u64 / 10_000);
        match self.link.recv(Instant::now() + wait)? {
            Message::Block(b) => Ok(b),
            Message::Line(s) => Err(ScpiError::Unexpected { asked: q.into(), got: s }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::row;
    use oos_link::Script;

    const NO_ERROR: &str = "> :SYSTem:ERRor?\n< 0,\"No error\"";

    fn scope(s: Script) -> Scope {
        Scope::new(Box::new(s))
    }

    #[test]
    fn identity_and_channels() {
        let mut s = scope(Script::new("> *IDN?\n< OOS,DHO924S,0,oos-ui 0.1.15"));
        let id = s.identity().expect("idn");
        assert_eq!((id.model.as_str(), id.version.as_str(), id.channels()), ("DHO924S", "0.1.15", 4));
        let two = Identity { model: "DHO802".into(), ..id };
        assert_eq!(two.channels(), 2);
    }

    #[test]
    fn a_set_reads_the_error_queue_and_a_refusal_is_an_error() {
        let r = row("chan.scale").expect("row");
        let mut s = scope(Script::new(&format!("> :CHANnel2:SCALe 5E-1\n{NO_ERROR}")));
        s.set(r, 2, &Value::Float(0.5)).expect("set");
        let mut s = scope(
            Script::new(
                "> :CHANnel2:SCALe 5E-1\n> :SYSTem:ERRor?\n< -224,\"Illegal parameter value\"\n".to_string().as_str(),
            )
            .then(NO_ERROR),
        );
        match s.set(r, 2, &Value::Float(0.5)) {
            Err(ScpiError::Refused(v)) => assert_eq!(v, vec![(-224, "Illegal parameter value".to_string())]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_query_answer_that_does_not_fit_is_an_error() {
        let r = row("trig.sweep").expect("row");
        let mut s = scope(Script::new("> :TRIGger:SWEep?\n< NORM"));
        assert_eq!(s.get(r, 1).expect("get"), Value::Word("NORM"));
        let mut s = scope(Script::new("> :TRIGger:SWEep?\n< BANANA"));
        assert!(matches!(s.get(r, 1), Err(ScpiError::Unexpected { .. })));
    }

    #[test]
    fn a_record_is_read_in_pages() {
        let pts = PAGE + 2;
        let setup = format!(
            "> :WAVeform:SOURce CHANnel1\n{NO_ERROR}\n> :WAVeform:MODE NORMal\n{NO_ERROR}\n\
             > :WAVeform:FORMat BYTE\n{NO_ERROR}\n> :WAVeform:STARt 1\n{NO_ERROR}\n\
             > :WAVeform:PREamble?\n< 0,0,{pts},1,1e-9,0,0,0.01,0,128\n\
             > :WAVeform:STARt 1;:WAVeform:STOP {PAGE}"
        );
        let s = Script::new(&setup)
            .block(":WAVeform:DATA?", vec![7; PAGE as usize])
            .then(&format!("> :WAVeform:STARt {};:WAVeform:STOP {pts}", PAGE + 1))
            .block(":WAVeform:DATA?", vec![9, 9]);
        let mut s = scope(s);
        let w = s.waveform(1).expect("waveform");
        assert_eq!(w.codes.len() as u64, pts);
        assert_eq!((w.codes[0], w.codes[pts as usize - 1]), (7, 9));
    }
}
