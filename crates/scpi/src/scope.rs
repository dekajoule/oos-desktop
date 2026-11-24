// SPDX-License-Identifier: GPL-2.0-or-later
//! The typed side of the scope: identity, the settings of the table, run control, records and the error queue.

use crate::commands::Row;
use crate::measures::{Measure, NO_VALUE};
use crate::{Preamble, Value, Waveform};
use oos_link::{Error as LinkError, Link, Message};
use std::time::{Duration, Instant};

/// How long a line answer may take.
const LINE_WAIT: Duration = Duration::from_secs(2);
/// Points read per `:WAVeform:DATA?` page.
pub const PAGE: u64 = 1_000_000;
/// Queries sent in one message at most (OOS docs/scpi.md: absolute headers, separated by semicolons; the answers come
/// in one line, separated the same way).
const CHAIN: usize = 12;
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

/// What the trigger is doing (`:TRIGger:STATus?`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerState {
    /// Stopped, also after a completed Single.
    Stop,
    /// Triggered.
    Triggered,
    /// Drawing without a trigger (Auto sweep).
    Auto,
    /// Armed, waiting for a trigger.
    Wait,
}

impl TriggerState {
    fn parse(a: &str) -> Option<TriggerState> {
        match a.trim().to_ascii_uppercase().as_str() {
            "STOP" => Some(TriggerState::Stop),
            "TD" => Some(TriggerState::Triggered),
            "AUTO" => Some(TriggerState::Auto),
            "WAIT" => Some(TriggerState::Wait),
            _ => None,
        }
    }
}

/// The state of the acquisition: what the trigger does, the sample rate and the memory depth; the last two None
/// when the scope answers something that is not a positive number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Acquisition {
    pub trigger: TriggerState,
    pub sample_rate: Option<f64>,
    pub depth: Option<f64>,
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

    /// The values of several settings, read with as few round trips as the scope allows: a link's round trip, not
    /// the scope's answer, is what a poll of many settings waits for.
    pub fn get_many(&mut self, which: &[(&Row, u8)]) -> Result<Vec<Value>, ScpiError> {
        let mut out = Vec::with_capacity(which.len());
        for chunk in which.chunks(CHAIN) {
            let qs: Vec<String> = chunk.iter().map(|(r, ch)| format!("{}?", r.header_for(*ch))).collect();
            let answers = self.query_chain(&qs)?;
            for ((r, _), (q, a)) in chunk.iter().zip(qs.iter().zip(answers)) {
                let v = Value::parse(&r.kind, &a);
                out.push(v.ok_or(ScpiError::Unexpected { asked: q.clone(), got: a })?);
            }
        }
        Ok(out)
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

    /// `:AUToscale`: the scope sets channels, time base and trigger to the signals.
    pub fn autoscale(&mut self) -> Result<(), ScpiError> {
        self.command(":AUToscale")
    }

    /// `:CLEar`: persistence and measurement statistics.
    pub fn clear(&mut self) -> Result<(), ScpiError> {
        self.command(":CLEar")
    }

    /// `:TFORce`: one record without waiting for the trigger.
    pub fn force(&mut self) -> Result<(), ScpiError> {
        self.command(":TFORce")
    }

    /// The trigger's state, the sample rate and the memory depth.
    pub fn acquisition(&mut self) -> Result<Acquisition, ScpiError> {
        let qs = [":TRIGger:STATus?", ":ACQuire:SRATe?", ":ACQuire:MDEPth?"].map(String::from);
        let a = self.query_chain(&qs)?;
        let trigger =
            TriggerState::parse(&a[0]).ok_or(ScpiError::Unexpected { asked: qs[0].clone(), got: a[0].clone() })?;
        let positive = |a: &str| a.parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0);
        Ok(Acquisition { trigger, sample_rate: positive(&a[1]), depth: positive(&a[2]) })
    }

    /// The screen record of channel `ch` (`:WAVeform:MODE NORMal`, BYTE), read in pages of [`PAGE`] points.
    pub fn waveform(&mut self, ch: u8) -> Result<Waveform, ScpiError> {
        self.command(&format!(
            ":WAVeform:SOURce CHANnel{ch};:WAVeform:MODE NORMal;:WAVeform:FORMat BYTE;:WAVeform:STARt 1"
        ))?;
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

    /// The scope's screen as PNG (`:DISPlay:DATA? PNG`).
    pub fn screenshot(&mut self) -> Result<Vec<u8>, ScpiError> {
        // the size is not known before: a screen of 1024 x 600 in PNG is well under 2 MB
        let png = self.block(":DISPlay:DATA? PNG", 2 << 20)?;
        if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            let got = format!("{} bytes that are not a PNG", png.len());
            return Err(ScpiError::Unexpected { asked: ":DISPlay:DATA? PNG".into(), got });
        }
        Ok(png)
    }

    /// Measurements of channels in as few round trips as the scope allows; None while the scope has no value for one
    /// (`9.9E37`).
    pub fn measure_many(&mut self, which: &[(&Measure, u8)]) -> Result<Vec<Option<f64>>, ScpiError> {
        let mut out = Vec::with_capacity(which.len());
        for chunk in which.chunks(CHAIN) {
            let qs: Vec<String> = chunk.iter().map(|(m, ch)| format!(":MEASure:ITEM? {},CHAN{ch}", m.item)).collect();
            for (q, a) in qs.iter().zip(self.query_chain(&qs)?) {
                out.push(no_value(q, &a)?);
            }
        }
        Ok(out)
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

    /// Queries sent as one message; their answers, one per query.
    fn query_chain(&mut self, qs: &[String]) -> Result<Vec<String>, ScpiError> {
        let line = self.query(&qs.join(";"))?;
        let parts: Vec<String> = line.split(';').map(|p| p.trim().to_string()).collect();
        if parts.len() != qs.len() {
            return Err(ScpiError::Unexpected { asked: qs.join(";"), got: line });
        }
        Ok(parts)
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

/// A measurement's answer: None for the scope's `9.9E37` (no value on this record).
fn no_value(q: &str, a: &str) -> Result<Option<f64>, ScpiError> {
    let x: f64 = a.trim().parse().map_err(|_| ScpiError::Unexpected { asked: q.into(), got: a.into() })?;
    Ok((x.is_finite() && x.abs() < NO_VALUE * 0.5).then_some(x))
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
        assert_eq!(s.get_many(&[(r, 1)]).expect("get"), vec![Value::Word("NORM")]);
        let mut s = scope(Script::new("> :TRIGger:SWEep?\n< BANANA"));
        assert!(matches!(s.get_many(&[(r, 1)]), Err(ScpiError::Unexpected { .. })));
    }

    #[test]
    fn the_acquisition_state_and_answers_that_do_not_fit() {
        let mut s =
            scope(Script::new("> :TRIGger:STATus?;:ACQuire:SRATe?;:ACQuire:MDEPth?\n< TD;1.250000E+09;1000000"));
        let a = s.acquisition().expect("state");
        assert_eq!(a, Acquisition { trigger: TriggerState::Triggered, sample_rate: Some(1.25e9), depth: Some(1e6) });
        let mut s = scope(Script::new("> :TRIGger:STATus?;:ACQuire:SRATe?;:ACQuire:MDEPth?\n< wait;nan;-1"));
        let a = s.acquisition().expect("state");
        assert_eq!((a.trigger, a.sample_rate, a.depth), (TriggerState::Wait, None, None));
        let mut s = scope(Script::new("> :TRIGger:STATus?;:ACQuire:SRATe?;:ACQuire:MDEPth?\n< RUNNING;1;1"));
        assert!(matches!(s.acquisition(), Err(ScpiError::Unexpected { .. })));
        let mut s = scope(Script::new("> :TRIGger:STATus?;:ACQuire:SRATe?;:ACQuire:MDEPth?\n< TD;1"));
        assert!(matches!(s.acquisition(), Err(ScpiError::Unexpected { .. })), "an answer missing");
    }

    #[test]
    fn settings_and_measurements_are_read_chained() {
        let (scale, sweep) = (row("chan.scale").expect("row"), row("trig.sweep").expect("row"));
        let mut s = scope(Script::new("> :CHANnel2:SCALe?;:TRIGger:SWEep?\n< 5.000000E-01;NORM"));
        let got = s.get_many(&[(scale, 2), (sweep, 1)]).expect("values");
        assert_eq!(got, vec![Value::Float(0.5), Value::Word("NORM")]);
        let mut s = scope(Script::new("> :CHANnel2:SCALe?;:TRIGger:SWEep?\n< 5.000000E-01;BANANA"));
        assert!(matches!(s.get_many(&[(scale, 2), (sweep, 1)]), Err(ScpiError::Unexpected { .. })));
        // more than a chain's worth go in two messages
        let many: Vec<_> = (0..CHAIN + 1).map(|_| (scale, 1)).collect();
        let first = [":CHANnel1:SCALe?"; CHAIN].join(";");
        let answers = ["1"; CHAIN].join(";");
        let mut s = scope(Script::new(&format!("> {first}\n< {answers}\n> :CHANnel1:SCALe?\n< 2")));
        assert_eq!(s.get_many(&many).expect("values").len(), CHAIN + 1);
        let (vpp, freq) = (crate::measures::measure("vpp").expect("vpp"), crate::measures::measure("freq").expect("f"));
        let mut s = scope(Script::new("> :MEASure:ITEM? VPP,CHAN1;:MEASure:ITEM? FREQUENCY,CHAN1\n< 2.5;9.9E37"));
        assert_eq!(s.measure_many(&[(vpp, 1), (freq, 1)]).expect("values"), vec![Some(2.5), None]);
    }

    #[test]
    fn a_screenshot_is_a_png() {
        let png = b"\x89PNG\r\n\x1a\nrest".to_vec();
        let mut s = scope(Script::default().block(":DISPlay:DATA? PNG", png.clone()));
        assert_eq!(s.screenshot().expect("png"), png);
        let mut s = scope(Script::default().block(":DISPlay:DATA? PNG", b"BM...".to_vec()));
        assert!(matches!(s.screenshot(), Err(ScpiError::Unexpected { .. })));
    }

    #[test]
    fn a_measurement_and_one_the_scope_has_not_got() {
        let freq = crate::measures::measure("freq").expect("freq");
        let mut s = scope(Script::new(
            "> :MEASure:ITEM? FREQUENCY,CHAN2\n< 1.000000E+03\n> :MEASure:ITEM? FREQUENCY,CHAN2\n< 9.9E37\n\
             > :MEASure:ITEM? FREQUENCY,CHAN2\n< what",
        ));
        assert_eq!(s.measure_many(&[(freq, 2)]).expect("value"), vec![Some(1000.0)]);
        assert_eq!(s.measure_many(&[(freq, 2)]).expect("none"), vec![None]);
        assert!(matches!(s.measure_many(&[(freq, 2)]), Err(ScpiError::Unexpected { .. })));
    }

    #[test]
    fn a_record_is_read_in_pages() {
        let pts = PAGE + 2;
        let setup = format!(
            "> :WAVeform:SOURce CHANnel1;:WAVeform:MODE NORMal;:WAVeform:FORMat BYTE;:WAVeform:STARt 1\n{NO_ERROR}\n\
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
